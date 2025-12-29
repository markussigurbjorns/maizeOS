#!/bin/bash -e

#clean
cargo clean
rm -rf iso

#build
RUSTFLAGS="-C link-arg=-Tlinker.ld" \
    cargo build --target target.json -Z build-std=core,compiler_builtins -Z build-std-features=compiler-builtins-mem --release

#post
mkdir -p iso/boot/grub
cp target/target/release/maizeOS iso/boot/kernel
cp grub.cfg iso/boot/grub/
grub2-mkrescue -o maizeOS.iso iso

#run
# qemu-system-x86_64 -cdrom maizeOS.iso -serial stdio -no-reboot -d int
