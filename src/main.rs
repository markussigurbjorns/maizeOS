#![no_std]
#![no_main]

mod sync;
mod vga_buffer;

use core::arch::global_asm;
use core::panic::PanicInfo;

use crate::vga_buffer::print;

global_asm!(include_str!("boot.S"));

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

#[repr(C, align(8))]
struct Multiboot2Header([u32; 6]);

#[unsafe(link_section = ".multiboot2")]
#[used]
static MULTIBOOT2_HEADER: Multiboot2Header = Multiboot2Header({
    let magic: u32 = 0xe85250d6;
    let arch: u32 = 0;
    let len: u32 = 24;
    let csum: u32 = 0u32.wrapping_sub(magic.wrapping_add(arch).wrapping_add(len));
    [magic, arch, len, csum, 0, 8]
});

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    print("Hello, World\n");
    print("MAGGI");
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
