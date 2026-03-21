mod cancellable_stream;
mod mut_waker;
mod ring_buf;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD;
pub use cancellable_stream::CancellableStream;
use rand::RngCore;
pub use ring_buf::RingBuf;
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
    let sql_name = re!(r"^[a-zA-Z_][a-zA-Z0-9_]+$");
    assert!(sql_name.is_match(table));
    assert!(sql_name.is_match(column));

    let sql = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {column} = $1)");
    let mut buffer = vec![0u8; bytes];

    loop {
        rand::thread_rng().fill_bytes(&mut buffer);
        let candidate = BASE64_URL_SAFE_NO_PAD.encode(&buffer);

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
