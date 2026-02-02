# maizeOS

A small x86_64 hobby kernel in Rust (`no_std`, `no_main`) booted via GRUB2 Multiboot2.

## What it does right now

- Boots through GRUB2 using a Multiboot2 header.
- Enters long mode from a 32-bit assembly stub.
- Initializes serial output (COM1) and VGA text output.
- Parses/dumps Multiboot2 info and memory map.
- Sets up GDT/TSS and IDT with handlers for `#BP`, `#UD`, `#DF`, `#GP`, `#PF`.
- Initializes a simple physical frame allocator.
- Maps a high virtual heap region and uses a bump allocator.

## Prerequisites

You need a nightly Rust toolchain and a few host tools:

- `rustup` with nightly toolchain
- `grub2-mkrescue` (or `grub-mkrescue`, distro-dependent)
- `xorriso` (used by grub mkrescue)
- `qemu-system-x86_64` (for local VM boot/testing)

Install Rust pieces:

```bash
rustup default nightly
rustup component add rust-src llvm-tools-preview
```

## Build

Quick build command (same flags used by `build.sh`):

```bash
RUSTFLAGS="-C link-arg=-Tlinker.ld" \
cargo build --target target.json \
  -Z build-std=core,alloc,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem \
  --release
```

Build + ISO image:

```bash
./build.sh
```

This creates `maizeOS.iso`.

## Run in QEMU

Run with serial logs to your terminal:

```bash
qemu-system-x86_64 \
  -machine q35 \
  -cpu qemu64 \
  -m 256M \
  -cdrom maizeOS.iso \
  -serial stdio \
  -no-reboot
```

## Expected output

- Serial should show boot progress (`maizeOS: entered rust_main`, MB2 dump, heap info, etc.).
- VGA should show:

```text
Welcome to MaizeOS
```

## Project layout

- `src/boot.S` - 32-bit entry, paging bootstrap, long-mode transition
- `src/main.rs` - kernel entry and init sequence
- `src/interrupts.S` - ISR stubs
- `src/idt.rs`, `src/gdt.rs` - descriptor tables and TSS
- `src/mb2.rs` - Multiboot2 parsing
- `src/frame_alloc.rs` - physical frame allocator
- `src/paging.rs` - 4 KiB page mapping helper
- `src/heap.rs` - bump global allocator
- `src/serial.rs`, `src/vga_buffer.rs` - output
- `linker.ld`, `target.json`, `grub.cfg` - build/boot config

## Debugging tips

- Prefer `-serial stdio` in QEMU to see panic/exception details.
- On exception, serial logs include vector, RIP, CS, RFLAGS, and PF decode.
- Keep `-no-reboot` so faults stay visible.
- If `grub2-mkrescue` is missing, try `grub-mkrescue` (command name varies by distro).

## Notes

- The heap allocator is currently a simple bump allocator (`dealloc` is a no-op).
- The build script does a clean build each time; remove `cargo clean` there if you want faster iteration.
