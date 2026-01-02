use core::alloc::{GlobalAlloc, Layout};

use crate::{serial_println, sync::spinlock::SpinLock};

struct Bump {
    start: usize,
    end: usize,
    next: usize,
}

impl Bump {
    const fn empty() -> Self {
        Self {
            start: 0,
            end: 0,
            next: 0,
        }
    }

    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start + size;
        self.next = start;
    }

    fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();

        let mut cur = self.next;
        cur = (cur + (align - 1)) & !(align - 1);

        let new_next = match cur.checked_add(size) {
            Some(v) => v,
            None => return core::ptr::null_mut(),
        };

        if new_next > self.end {
            return core::ptr::null_mut();
        }

        self.next = new_next;
        cur as *mut u8
    }
}

pub struct KernelAlloc {
    bump: SpinLock<Bump>,
}

impl KernelAlloc {
    pub const fn new() -> Self {
        Self {
            bump: SpinLock::new(Bump::empty()),
        }
    }

    pub fn init(&self, heap_start: usize, heap_size: usize) {
        let mut b = self.bump.lock();
        b.init(heap_start, heap_size);
    }
}

unsafe impl GlobalAlloc for KernelAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut b = self.bump.lock();
        b.alloc(layout)
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOC: KernelAlloc = KernelAlloc::new();

#[alloc_error_handler]
fn alloc_error(layout: Layout) -> ! {
    serial_println!("ALLOC ERROR: {:?}", layout);
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

pub fn init(heap_start: usize, heap_size: usize) {
    ALLOC.init(heap_start, heap_size);
}
