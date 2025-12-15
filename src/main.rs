#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

#[unsafe(link_section = ".multiboot2")]
#[used]
static MULTIBOOT2_HEADER: [u32; 8] = [0xe85250d6, 0, 32, !(0xe85250d6 + 0 + 32) + 1, 0, 0, 0, 0];

#[used]
static HELLO: &[u8] = b"Hello World!";

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let vga_buffer = 0xb8000 as *mut u8;

    for (i, &byte) in HELLO.iter().enumerate() {
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
