use std::mem;
use std::task::Waker;

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
        // using mem::replace instead of assignment seems to produce better assembly.
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
