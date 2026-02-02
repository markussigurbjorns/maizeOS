use crate::{frame_alloc::FrameAllocator, serial_println};

const PAGE_SIZE: u64 = 4096;

const PTE_P: u64 = 1 << 0;
const PTE_W: u64 = 1 << 1;
const _PTE_U: u64 = 1 << 2;
const PTE_PS: u64 = 1 << 7;

fn align_down(x: u64, a: u64) -> u64 {
    x & !(a - 1)
}

fn pml4_index(v: u64) -> usize {
    ((v >> 39) & 0x1FF) as usize
}
fn pdpt_index(v: u64) -> usize {
    ((v >> 30) & 0x1FF) as usize
}
fn pd_index(v: u64) -> usize {
    ((v >> 21) & 0x1FF) as usize
}
fn pt_index(v: u64) -> usize {
    ((v >> 12) & 0x1FF) as usize
}

type PageTable = [u64; 512];

unsafe fn read_cr3() -> u64 {
    let cr3: u64;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
    }
    cr3
}

unsafe fn invlpg(addr: u64) {
    unsafe {
        core::arch::asm!("invlpg [{}]",in(reg) addr, options(nomem, nostack, preserves_flags))
    }
}

fn zero_page_table(phys: u64) {
    let pt = phys as *mut PageTable; // identity-mapped assumption
    unsafe {
        core::ptr::write_bytes(pt as *mut u8, 0, PAGE_SIZE as usize);
    }
}

fn pte_addr(pte: u64) -> u64 {
    pte & 0x000F_FFFF_FFFF_F000
}

fn ensure_table(fa: &mut FrameAllocator, parent: &mut PageTable, idx: usize, flags: u64) -> u64 {
    let entry = parent[idx];
    if (entry & PTE_P) != 0 {
        //present
        return pte_addr(entry);
    }

    let new_phys = fa.alloc_frame().expect("out of frames for page tables");
    zero_page_table(new_phys);
    parent[idx] = new_phys | flags | PTE_P;
    new_phys
}

/// Map a single 4KiB page: virt -> phys with flags (P/W/U).
/// Requires that page tables are identity-mapped (true in current setup).
pub fn map_4k(fa: &mut FrameAllocator, virt: u64, phys: u64, flags: u64) {
    let virt = align_down(virt, PAGE_SIZE);
    let phys = align_down(phys, PAGE_SIZE);

    unsafe {
        let cr3 = read_cr3();
        let pml4_phys = pte_addr(cr3);
        let pml4 = &mut *(pml4_phys as *mut PageTable);

        let pdpt_phys = ensure_table(fa, pml4, pml4_index(virt), PTE_W);
        let pdpt = &mut *(pdpt_phys as *mut PageTable);

        let pd_phys = ensure_table(fa, pdpt, pdpt_index(virt), PTE_W);
        let pd = &mut *(pd_phys as *mut PageTable);

        // If there is a 2MiB huge page already mapped here, we should NOT overwrite it.
        // boot mapping uses 2MiB pages for low memory. So if we try to map inside that range,
        // we'd need to split the huge page first.
        let pde = pd[pd_index(virt)];
        if (pde & PTE_P) != 0 && (pde & PTE_PS) != 0 {
            serial_println!("paging: cannot map_4k inside 2MiB mapping yet (need split)");
            loop {
                core::arch::asm!("hlt");
            }
        }

        let pt_phys = ensure_table(fa, pd, pd_index(virt), PTE_W);
        let pt = &mut *(pt_phys as *mut PageTable);

        pt[pt_index(virt)] = phys | flags | PTE_P;

        invlpg(virt);
    }
}
