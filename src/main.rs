#![no_std]
#![no_main]

mod vga_buffer;

use core::{panic::PanicInfo, ptr};

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
static MULTIBOOT2_HEADER: [u32; 6] = {
    let magic: u32 = 0xe85250d6;
    let arch: u32 = 0;
    let len: u32 = 24;
    let csum: u32 = 0u32.wrapping_sub(magic.wrapping_add(arch).wrapping_add(len));
    [magic, arch, len, csum, 0, 8]
};

static HELLO: &[u8] = b"Hello World!";

#[unsafe(no_mangle)]
pub extern "C" fn _start(_multiboot_info: usize) -> ! {
    let vga = 0xb8000 as *mut u16;

    for (i, &ch) in HELLO.iter().enumerate() {
        unsafe {
            ptr::write_volatile(vga.add(i), (0x0Bu16 << 8) | (ch as u16));
        }
    }

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
