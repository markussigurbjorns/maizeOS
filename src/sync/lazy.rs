use core::sync::atomic::{AtomicU8, Ordering};
use core::{cell::UnsafeCell, mem::MaybeUninit};

pub struct Lazy<T, F = fn() -> T> {
    state: AtomicU8, // 0 = uninit, 1 = initing, 2 = ready
    value: UnsafeCell<MaybeUninit<T>>,
    init: UnsafeCell<Option<F>>,
}

unsafe impl<T: Send + Sync, F: Send> Sync for Lazy<T, F> {}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    pub const fn new(init: F) -> Self {
        Self {
            state: AtomicU8::new(0),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            init: UnsafeCell::new(Some(init)),
        }
    }

    pub fn get(&self) -> &T {
        if self.state.load(Ordering::Acquire) != 2 {
            self.init_slow();
        }
        unsafe { (*self.value.get()).assume_init_ref() }
    }

    fn init_slow(&self) {
        if self
            .state
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let init = unsafe { (*self.init.get()).take().unwrap() };
            let value = init();
            unsafe { (*self.value.get()).write(value) };
            self.state.store(2, Ordering::Release);
        } else {
            // someone else is initializing; wait
            while self.state.load(Ordering::Acquire) != 2 {
                core::hint::spin_loop();
            }
        }
    }
}
