use core::u64;

use crate::{
    mb2::{self, Mb2MmapTag},
    serial_println,
};

const PAGE_SIZE: u64 = 4096;

fn align_up(x: u64, a: u64) -> u64 {
    (x + (a - 1)) & !(a - 1)
}

fn align_down(x: u64, a: u64) -> u64 {
    x & !(a - 1)
}

pub struct FrameAllocator {
    mmap_ptr: *const mb2::Mb2MmapTag,
    entries_start: u64,
    entries_end: u64,
    entry_size: u64,

    cur_entry_ptr: u64,
    cur_frame: u64,
    cur_region_end: u64,

    kernel_start: u64,
    kernel_end: u64,
    mb2_start: u64,
    mb2_end: u64,
}

impl FrameAllocator {
    pub fn init(mb2_info_phys: u64, kernel_start: u64, kernel_end: u64) -> Option<Self> {
        let mmap = mb2::get_mmap_tag(mb2_info_phys as usize)? as *const Mb2MmapTag;
        let mmap_ref = unsafe { &*mmap };

        let tag_ptr = mmap as u64;
        let tag_size = mmap_ref.tag.size as u64;

        let entries_start = tag_ptr + core::mem::size_of::<mb2::Mb2MmapTag>() as u64;
        let entries_end = tag_ptr + tag_size;

        let entry_size = mmap_ref.entry_size as u64;

        // total_size is first u32
        let mb2_info_total_size = unsafe { *(mb2_info_phys as *const u32) } as u64;
        let mb2_start = mb2_info_phys;
        let mb2_end = mb2_info_phys + mb2_info_total_size;

        let mut fa = Self {
            mmap_ptr: mmap,
            entries_start,
            entries_end,
            entry_size,
            cur_entry_ptr: entries_start,
            cur_frame: 0,
            cur_region_end: 0,
            kernel_start,
            kernel_end,
            mb2_start,
            mb2_end,
        };

        fa.advance_to_next_usable_reagion();

        Some(fa)
    }

    pub fn alloc_frame(&mut self) -> Option<u64> {
        loop {
            if self.cur_frame == 0 || self.cur_frame >= self.cur_region_end {
                if !self.advance_to_next_usable_reagion() {
                    return None;
                }
            }

            let frame = self.cur_frame;
            self.cur_frame += PAGE_SIZE;

            if self.frame_is_forbidden(frame) {
                continue;
            }

            return Some(frame);
        }
    }

    fn frame_is_forbidden(&mut self, frame: u64) -> bool {
        // avoid low memory for now (< 1MiB)
        if frame < 0x0010_0000 {
            return true;
        }

        // avoid kernel image
        if frame >= align_down(self.kernel_start, PAGE_SIZE)
            && frame < align_up(self.kernel_end, PAGE_SIZE)
        {
            return true;
        }

        // avoid multiboot info blob
        if frame >= align_down(self.mb2_start, PAGE_SIZE)
            && frame < align_up(self.mb2_end, PAGE_SIZE)
        {
            return true;
        }

        false
    }

    fn advance_to_next_usable_reagion(&mut self) -> bool {
        while self.cur_entry_ptr + self.entry_size <= self.entries_end {
            let ent = unsafe { &*(self.cur_entry_ptr as *const mb2::Mb2MmapEntry) };

            self.cur_entry_ptr += self.entry_size;

            // Type 1 = available RAM
            if ent.entry_type != 1 {
                continue;
            }

            let base = ent.base_addr;
            let end = ent.base_addr.saturating_add(ent.length);

            // Skip anything completely below 1MiB
            if end <= 0x0010_0000 {
                continue;
            }

            let region_start = align_up(base.max(0x0010_0000), PAGE_SIZE);
            let region_end = align_down(end, PAGE_SIZE);

            if region_end <= region_start {
                continue;
            }

            self.cur_frame = region_start;
            self.cur_region_end = region_end;

            serial_println!(
                "FA: using region base={:#x}..{:#x}",
                self.cur_frame,
                self.cur_region_end
            );
            return true;
        }
        false
    }
}
