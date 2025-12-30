use core::mem::size_of;

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: u64,
}

#[repr(C, packed)]
pub struct Tss64 {
    _rsv0: u32,
    pub rsp: [u64; 3],
    _rsv1: u64,
    pub ist: [u64; 7],
    _rsv2: u64,
    _rsv3: u16,
    pub iopb_offset: u16,
}

impl Tss64 {
    const fn new() -> Self {
        Self {
            _rsv0: 0,
            rsp: [0; 3],
            _rsv1: 0,
            ist: [0; 7],
            _rsv2: 0,
            _rsv3: 0,
            iopb_offset: size_of::<Tss64>() as u16,
        }
    }
}

const DF_STACK_SIZE: usize = 4096 * 4;

// A dedicated stack for double faults (IST1)
#[repr(align(16))]
struct DFSTACK<T>(T);

static mut DF_STACK: DFSTACK<[u8; DF_STACK_SIZE]> = DFSTACK([0; DF_STACK_SIZE]);

static mut TSS: Tss64 = Tss64::new();

// GDT layout:
// 0: null
// 1: code (0x08)
// 2: data (0x10)
// 3-4: TSS descriptor (0x18 selector)
static mut GDT: [u64; 5] = [0; 5];

const GDT_CODE: u64 = 0x00AF9A000000FFFF;
const GDT_DATA: u64 = 0x00AF92000000FFFF;

fn make_tss_descriptor(base: u64, limit: u32) -> (u64, u64) {
    // 64-bit TSS descriptor is 16 bytes split across two u64s.
    // Type = 0x9 (available 64-bit TSS), Present=1.
    let mut low: u64 = 0;
    low |= (limit as u64) & 0xFFFF;
    low |= (base & 0xFFFF) << 16;
    low |= ((base >> 16) & 0xFF) << 32;
    low |= (0x89u64) << 40; // P=1, DPL=0, Type=0x9
    low |= ((limit as u64 >> 16) & 0xF) << 48;
    low |= ((base >> 24) & 0xFF) << 56;

    let high: u64 = base >> 32;
    (low, high)
}

pub fn init(stack_top: u64) {
    unsafe {
        // IST1 = DF stack top
        let df_stack_top = {
            let bytes: *const u8 =
                core::ptr::addr_of!((*core::ptr::addr_of!(DF_STACK)).0) as *const u8;
            bytes.add(DF_STACK_SIZE) as u64 & !0xF
        };
        TSS.ist[0] = df_stack_top; // IST1
        TSS.rsp[0] = stack_top; // RSP0 (future user->kernel transitions)

        GDT[0] = 0;
        GDT[1] = GDT_CODE;
        GDT[2] = GDT_DATA;

        let tss_base = core::ptr::addr_of!(TSS) as u64;
        let tss_limit = (size_of::<Tss64>() - 1) as u32;
        let (tss_lo, tss_hi) = make_tss_descriptor(tss_base, tss_limit);
        GDT[3] = tss_lo;
        GDT[4] = tss_hi;

        let gdtr = DescriptorTablePointer {
            limit: (size_of::<[u64; 5]>() - 1) as u16,
            base: core::ptr::addr_of!(GDT) as u64,
        };

        // Load the new GDT (CS stays 0x08, but we reload data segs).
        core::arch::asm!("lgdt [{}]", in(reg) &gdtr, options(readonly, nostack, preserves_flags));

        // Reload data segments to match the new GDT.
        core::arch::asm!(
            "mov ax, 0x10",
            "mov ds, ax",
            "mov es, ax",
            "mov ss, ax",
            "mov fs, ax",
            "mov gs, ax",
            options(nostack, preserves_flags)
        );

        // Load Task Register with TSS selector (0x18 = entry 3).
        core::arch::asm!("ltr ax", in("ax") 0x18u16, options(nostack, preserves_flags));
    }
}
