ENTRY(_start)

SECTIONS {
    . = 1M;
    __kernel_start = .;

    .text : {
        *(.multiboot2)
        *(.text*)
    }

    .rodata : {
        *(.rodata*)
    }

    .data : {
        *(.data*)
    }

    .bss : {
        *(.bss*)
        *(COMMON)
    }
    __kernel_end = .;
}
