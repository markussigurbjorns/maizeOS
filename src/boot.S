.section .text
.global _start
.type _start, @function

.code32
_start:
    cli
    mov esp, offset stack_top
    mov ebp, esp

    call setup_long_mode

.hang:
    hlt
    jmp .hang

setup_long_mode:
    lea eax, pml4_table
    lea edx, pdpt_table
    or edx, 0x3
    mov dword ptr [eax], edx
    mov dword ptr [eax + 4], 0

    lea eax, pdpt_table
    lea edx, pd_table
    or edx, 0x3
    mov dword ptr [eax], edx
    mov dword ptr [eax + 4], 0

    lea edi, pd_table
    xor ebx, ebx
    mov ecx, 512
.fill_pd:
    mov eax, ebx
    or eax, 0x83
    mov dword ptr [edi], eax
    mov dword ptr [edi + 4], 0
    add ebx, 0x200000
    add edi, 8
    loop .fill_pd

    lea eax, pml4_table
    mov cr3, eax

    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    lgdt [gdt64_ptr]
    push 0x08
    lea eax, long_mode_entry
    push eax
    retf

.code64
long_mode_entry:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax
    lea rsp, [stack_top]

    call rust_main

1:
    hlt
    jmp 1b

.section .rodata
.align 8
gdt64:
    .quad 0
    .quad 0x00AF9A000000FFFF
    .quad 0x00AF92000000FFFF
gdt64_end:

gdt64_ptr:
    .word gdt64_end - gdt64 - 1
    .long gdt64

.section .bss
.align 4096
pml4_table:
    .skip 4096
pdpt_table:
    .skip 4096
pd_table:
    .skip 4096

.align 16
stack_bottom:
    .skip 16384
stack_top:
