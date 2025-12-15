#!/bin/bash -e

#clean
cargo clean
rm -rf iso

#build
cargo build --target target.json -Z build-std=core --release

#post
mkdir -p iso/boot/grub
cp target/target/release/maizeOS iso/boot/kernel
cp grub.cfg iso/boot/grub/
grub2-mkrescue -o maizeOS.iso iso

#run
# qemu-system-x86_64 -cdrom maizeOS.iso
