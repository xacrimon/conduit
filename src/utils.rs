use std::mem;
use std::task::Waker;

use axum::response::{IntoResponse, Response};
use base64::Engine as _;
use base64::engine::general_purpose;
use rand::RngCore;
use sqlx::PgTransaction;

macro_rules! _re {
    ($re_expr:literal) => {{
        static RE: ::std::sync::LazyLock<::regex_lite::Regex> =
            ::std::sync::LazyLock::new(|| ::regex_lite::Regex::new($re_expr).unwrap());

        &*RE
    }};
}

pub(crate) use _re as re;

pub async fn unique_string(
    txn: &mut PgTransaction<'_>,
    table: &str,
    column: &str,
    bytes: usize,
) -> String {
    let sql = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {column} = $1)");
    let mut buffer = vec![0u8; bytes];

    loop {
        rand::thread_rng().fill_bytes(&mut buffer);
        let candidate = general_purpose::URL_SAFE_NO_PAD.encode(&buffer);

        let exists = sqlx::query_scalar::<_, bool>(&sql)
            .bind(&candidate)
            .fetch_one(&mut **txn)
            .await
            .unwrap();

        if !exists {
            return candidate;
        }
    }
}

pub struct MutWaker {
    registered: bool,
    waker: Waker,
}

impl MutWaker {
    pub fn new() -> Self {
        Self {
            registered: false,
            waker: Waker::noop().clone(),
        }
    }

    pub fn notify(&mut self) {
        if self.registered {
            self.registered = false;
            self.waker.wake_by_ref();
        }
    }

    pub fn notify_by_val(&mut self) {
        if self.registered {
            self.unregister_inner().wake();
        }
    }

    pub fn register(&mut self, waker: &Waker) {
        self.registered = true;
        if self.waker.will_wake(waker) {
            return;
        }
        // outlined to avoid a bunch of register fuss.
        self.register_slow(waker);
    }

    #[cold]
    #[inline(never)]
    fn register_slow(&mut self, waker: &Waker) {
        // using mem::replace instead of assignment seems to produce much more optimal assembly.
        _ = mem::replace(&mut self.waker, waker.clone());
    }

    pub fn unregister(&mut self) {
        drop(self.unregister_inner());
    }

    fn unregister_inner(&mut self) -> Waker {
        self.registered = false;
        mem::replace(&mut self.waker, Waker::noop().clone())
    }
}
