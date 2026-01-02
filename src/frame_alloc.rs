use crate::{
    mb2::{self, Mb2MmapTag},
    serial_println,
};

pub const PAGE_SIZE: u64 = 4096;

fn align_up(x: u64, a: u64) -> u64 {
    (x + (a - 1)) & !(a - 1)
}

fn align_down(x: u64, a: u64) -> u64 {
    x & !(a - 1)
}

pub struct FrameAllocator {
    _mmap_ptr: *const mb2::Mb2MmapTag,
    _entries_start: u64,
    entries_end: u64,
    entry_size: u64,

    cur_entry_ptr: u64,
    cur_frame: u64,
    cur_region_end: u64,

    _kernel_start: u64,
    _kernel_end: u64,
    min_start: u64,
    _mb2_start: u64,
    _mb2_end: u64,
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

        let min_start = align_up(core::cmp::max(kernel_end, mb2_end), PAGE_SIZE);

        let mut fa = Self {
            _mmap_ptr: mmap,
            _entries_start: entries_start,
            entries_end,
            entry_size,
            cur_entry_ptr: entries_start,
            cur_frame: 0,
            cur_region_end: 0,
            _kernel_start: kernel_start,
            _kernel_end: kernel_end,
            min_start,
            _mb2_start: mb2_start,
            _mb2_end: mb2_end,
        };

        fa.advance_to_next_usable_region();

        Some(fa)
    }

    pub fn alloc_frame(&mut self) -> Option<u64> {
        loop {
            if self.cur_frame == 0 || self.cur_frame >= self.cur_region_end {
                if !self.advance_to_next_usable_region() {
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

    fn frame_is_forbidden(&self, frame: u64) -> bool {
        // avoid low memory for now (< 1MiB)
        if frame < 0x0010_0000 {
            return true;
        }

        false
    }

    fn advance_to_next_usable_region(&mut self) -> bool {
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

            let region_start_tmp = align_up(base.max(0x0010_0000), PAGE_SIZE);
            let region_start = core::cmp::max(region_start_tmp, self.min_start);

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
