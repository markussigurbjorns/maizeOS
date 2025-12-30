.section .text
.code64

// Common convention we’re using:
// - For exceptions with an error code: CPU pushes [error][RIP][CS][RFLAGS]
// - For exceptions without an error code: CPU pushes [RIP][CS][RFLAGS]
// We’ll normalize by ensuring we always pass:
//   rdi = vector
//   rsi = error (0 if none)
//   rdx = pointer to RIP in the saved frame

.macro ISR_NOERR name, vec
.global \name
.type \name, @function
\name:
    // Stack: [RIP][CS][RFLAGS]
    xor rsi, rsi            // error = 0
    mov rdi, \vec           // vector
    lea rdx, [rsp + 0]      // &RIP
    call rust_exception_handler
1:  hlt
    jmp 1b
.endm

.macro ISR_ERR name, vec
.global \name
.type \name, @function
\name:
    // Stack: [error][RIP][CS][RFLAGS]
    mov rdi, \vec           // vector
    mov rsi, qword ptr [rsp + 0]   // error
    lea rdx, [rsp + 8]      // &RIP
    call rust_exception_handler
1:  hlt
    jmp 1b
.endm

# Breakpoint (vector 3)
.global isr_bp
.type isr_bp, @function
isr_bp:
    push rax
    push rcx
    push rdx
    push rsi
    push rdi
    push r8
    push r9
    push r10
    push r11

    // Stack: [r11..rax][RIP][CS][RFLAGS]
    lea rdi, [rsp + 9*8]      // arg: &RIP
    call rust_breakpoint_handler

    pop r11
    pop r10
    pop r9
    pop r8
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rax
    iretq

// #UD Invalid Opcode (vector 6)
ISR_NOERR isr_ud, 6

// #DF Double Fault (vector 8)
ISR_ERR isr_df, 8

// #GP General Protection (vector 13)
ISR_ERR   isr_gp, 13

// #PF Page Fault (vector 14) - error code
ISR_ERR   isr_pf, 14
