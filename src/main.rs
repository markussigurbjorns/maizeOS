#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

mod frame_alloc;
mod gdt;
mod heap;
mod idt;
mod mb2;
mod serial;
mod sync;
mod vga_buffer;

use core::arch::global_asm;
use core::panic::PanicInfo;
use core::usize;

extern crate alloc;
use alloc::vec::Vec;

use crate::frame_alloc::PAGE_SIZE;
use crate::vga_buffer::print;

global_asm!(include_str!("boot.S"));
global_asm!(include_str!("interrupts.S"));

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::init();
    serial_println!("KERNEL PANIC: {}", info);
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_exception_handler(vector: u64, error: u64, frame_rip_ptr: *const u64) -> ! {
    // frame_rip_ptr points to: [RIP, CS, RFLAGS] (ring0 case)
    let rip = unsafe { *frame_rip_ptr.add(0) };
    let cs = unsafe { *frame_rip_ptr.add(1) };
    let rflags = unsafe { *frame_rip_ptr.add(2) };

    serial_println!("");
    serial_println!("=== EXCEPTION ===");
    serial_println!("vector = {}  error = {:#x}", vector, error);
    serial_println!("RIP    = {:#016x}", rip);
    serial_println!("CS     = {:#x}", cs);
    serial_println!("RFLAGS = {:#016x}", rflags);

    match vector {
        3 => serial_println!("#BP Breakpoint"),
        6 => serial_println!("#UD Invalid Opcode"),
        8 => serial_println!("#DF Double Fault (IST1)"),
        13 => serial_println!("#GP General Protection Fault"),
        14 => {
            serial_println!("#PF Page Fault");

            let cr2: u64;
            unsafe {
                core::arch::asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack, preserves_flags));
            }
            serial_println!("CR2 (fault addr) = {:#016x}", cr2);

            // Decode PF error code bits
            let p = (error & (1 << 0)) != 0;
            let wr = (error & (1 << 1)) != 0;
            let us = (error & (1 << 2)) != 0;
            let rsvd = (error & (1 << 3)) != 0;
            let id = (error & (1 << 4)) != 0;

            serial_println!(
                "PF reason: {} | access={} | mode={} | rsvd={} | ifetch={}",
                if p {
                    "protection violation"
                } else {
                    "not-present page"
                },
                if wr { "write" } else { "read" },
                if us { "user" } else { "kernel" },
                rsvd,
                id
            );
        }
        _ => {}
    }

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_breakpoint_handler(frame_rip_ptr: *const u64) {
    let rip = unsafe { *frame_rip_ptr.add(0) };
    serial_println!("");
    serial_println!("=== #BP Breakpoint ===");
    serial_println!("RIP = {:#016x}", rip);
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
pub extern "C" fn rust_main(mb2_info: u32) -> ! {
    serial::init();
    serial_println!("maizeOS: entered rust_main");
    serial_println!("mb2_info ptr = {:#x}", mb2_info);
    mb2::dump(mb2_info as usize);

    unsafe extern "C" {
        static __kernel_start: u8;
        static __kernel_end: u8;
    }

    let kstart = unsafe { &__kernel_start as *const u8 as u64 };
    let kend = unsafe { &__kernel_end as *const u8 as u64 };
    serial_println!("kernel range: {:#x}..{:#x}", kstart, kend);

    let mut fa = frame_alloc::FrameAllocator::init(mb2_info as u64, kstart, kend)
        .expect("failed to initialize FrameAllocator");

    const HEAP_PAGES: usize = 1024;
    let mut heap_start: u64 = 0;

    for i in 0..HEAP_PAGES {
        let f = fa.alloc_frame().expect("out of frames");
        if i == 0 {
            heap_start = f;
        } else {
            // ensure contiguous (because bump heap assumes a contiguous range)
            let expected = heap_start + (i as u64) * 4096;
            if f != expected {
                panic!(
                    "heap frames not contiguous: got {:#x}, expected {:#x}",
                    f, expected
                );
            }
        }
    }

    let heap_size = HEAP_PAGES * PAGE_SIZE as usize;
    heap::init(heap_start as usize, heap_size);
    serial_println!("heap: start={:#x} size={} bytes", heap_start, heap_size);

    let mut v = Vec::new();
    for i in 0..16 {
        v.push(i);
    }
    serial_println!("heap test vec len={}", v.len());

    unsafe extern "C" {
        static stack_top: u8;
    }

    let stack_top_addr = unsafe { &stack_top as *const u8 as u64 };
    gdt::init(stack_top_addr);
    serial_println!("GDT+TSS loaded (IST1 for #DF)");

    idt::init();
    serial_println!("IDT loaded (#BP/#UD/#DF/#GP/#PF)");

    print("Welcome to MaizeOS");

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
