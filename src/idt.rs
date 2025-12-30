use core::mem::size_of;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    zero: u32,
}

impl IdtEntry {
    const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            zero: 0,
        }
    }

    fn set_handler(&mut self, handler: unsafe extern "C" fn()) {
        let addr = handler as u64;
        self.offset_low = addr as u16;
        self.selector = 0x08; // your 64-bit code segment selector
        self.ist = 0; // no IST for now
        self.type_attr = 0x8E; // Present=1, DPL=0, Type=0xE (interrupt gate)
        self.offset_mid = (addr >> 16) as u16;
        self.offset_high = (addr >> 32) as u32;
        self.zero = 0;
    }

    fn set_handler_with_ist(&mut self, handler: unsafe extern "C" fn(), ist_index: u8) {
        let addr = handler as u64;
        self.offset_low = addr as u16;
        self.selector = 0x08;
        self.ist = ist_index & 0x7; // 1..7 (0 means “don’t use IST”)
        self.type_attr = 0x8E;
        self.offset_mid = (addr >> 16) as u16;
        self.offset_high = (addr >> 32) as u32;
        self.zero = 0;
    }
}

#[repr(C, packed)]
struct Idtr {
    limit: u16,
    base: u64,
}

static mut IDT: [IdtEntry; 256] = [IdtEntry::missing(); 256];

unsafe extern "C" {
    fn isr_bp();
    fn isr_ud();
    fn isr_df();
    fn isr_gp();
    fn isr_pf();
}

pub fn init() {
    unsafe {
        let idt_pointer: *mut IdtEntry = core::ptr::addr_of_mut!(IDT) as *mut IdtEntry;

        (*idt_pointer.add(3)).set_handler(isr_bp);
        (*idt_pointer.add(6)).set_handler(isr_ud);
        (*idt_pointer.add(8)).set_handler_with_ist(isr_df, 1);
        (*idt_pointer.add(13)).set_handler(isr_gp);
        (*idt_pointer.add(14)).set_handler(isr_pf);

        let idtr = Idtr {
            limit: (size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: idt_pointer as u64,
        };

        core::arch::asm!("lidt [{}]", in(reg) &idtr, options(readonly, nostack, preserves_flags));
    }
}
