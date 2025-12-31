use crate::serial_println;

#[repr(C)]
struct Mb2InfoHeader {
    total_size: u32,
    reserved: u32,
}

#[repr(C)]
pub struct Mb2TagHeader {
    pub mb_type: u32,
    pub size: u32,
}

#[repr(C)]
pub struct Mb2MmapTag {
    pub tag: Mb2TagHeader,
    pub entry_size: u32,
    pub entry_version: u32,
    //entries
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Mb2MmapEntry {
    pub base_addr: u64,
    pub length: u64,
    pub entry_type: u32,
    pub reserved: u32,
}

fn align_up_8(x: usize) -> usize {
    (x + 7) & !7
}

pub fn get_mmap_tag(mb2_info_phys: usize) -> Option<&'static Mb2MmapTag> {
    if mb2_info_phys == 0 {
        serial_println!("MB2: null pointer");
        return None;
    }
    let info = unsafe { &*(mb2_info_phys as *const Mb2InfoHeader) };

    let start = mb2_info_phys;
    let end = start + info.total_size as usize;

    let mut p = start + core::mem::size_of::<Mb2InfoHeader>();
    while p + core::mem::size_of::<Mb2InfoHeader>() <= end {
        let tag = unsafe { &*(p as *const Mb2TagHeader) };

        if tag.size < 8 {
            serial_println!("MB2: ERROR tag size < 8 at {:#x}", p);
            break;
        }

        if tag.mb_type == 0 && tag.size == 8 {
            break;
        }

        if tag.mb_type == 6 {
            return Some(unsafe { &*(p as *const Mb2MmapTag) });
        }

        let next = p + align_up_8(tag.size as usize);

        if next <= p {
            serial_println!("MB2: ERROR tag pointer overflow");
            break;
        }
        p = next;
    }
    None
}

pub fn dump(mb2_info_phys: usize) {
    if mb2_info_phys == 0 {
        serial_println!("MB2: null pointer");
        return;
    }

    let info = unsafe { &*(mb2_info_phys as *const Mb2InfoHeader) };

    serial_println!("MB2: info @ {:#x}", mb2_info_phys);
    serial_println!("MB2: total size = {}", info.total_size);
    serial_println!("MB2: reserved = {}", info.reserved);

    if info.reserved != 0 {
        serial_println!("MB2: WARNING reserved !=0 (unexpecteda)");
    }

    if info.total_size < core::mem::size_of::<Mb2InfoHeader>() as u32 {
        serial_println!("MB2: ERROR total size too small");
        return;
    }

    let start = mb2_info_phys;
    let end = start + info.total_size as usize;

    let mut p = start + core::mem::size_of::<Mb2InfoHeader>();

    let mut saw_mmap = false;

    while p + core::mem::size_of::<Mb2InfoHeader>() <= end {
        let tag = unsafe { &*(p as *const Mb2TagHeader) };

        if tag.size < 8 {
            serial_println!("MB2: ERROR tag size < 8 at {:#x}", p);
            break;
        }

        if tag.mb_type == 0 && tag.size == 8 {
            serial_println!("MB2: end tag");
            break;
        }

        serial_println!("MB2: tag typ={} size={} @ {:#x}", tag.mb_type, tag.size, p);

        if tag.mb_type == 6 {
            saw_mmap = true;
            dump_mmap_tag(p, end);
        }

        let next = p + align_up_8(tag.size as usize);

        if next <= p {
            serial_println!("MB2: ERROR tag pointer overflow");
            break;
        }
        p = next;
    }
    if !saw_mmap {
        serial_println!("MB2: WARNING no memory map tag (type 6) found");
    }
}

fn dump_mmap_tag(tag_ptr: usize, info_end: usize) {
    let mmap = unsafe { &*(tag_ptr as *const Mb2MmapTag) };

    serial_println!(
        "MB2: mmap entry_size={} entry_version={}",
        mmap.entry_size,
        mmap.entry_version
    );

    let tag_size = mmap.tag.size as usize;
    if tag_ptr + tag_size > info_end {
        serial_println!("MB2: ERROR mmap tag overruns info_end");
        return;
    }

    if mmap.entry_size < core::mem::size_of::<Mb2MmapEntry>() as u32 {
        serial_println!(
            "MB2: ERROR mmap entry_size {} < {}",
            mmap.entry_size,
            core::mem::size_of::<Mb2MmapEntry>()
        );
        return;
    }

    // Entries begin after Mb2MmapTag struct (which is 16 bytes: header(8)+entry_size(4)+entry_version(4))
    let mut e = tag_ptr + core::mem::size_of::<Mb2MmapTag>();
    let entries_end = tag_ptr + tag_size;
    let mut idx: usize = 0;
    while e + (mmap.entry_size as usize) <= entries_end {
        let ent = unsafe { &*(e as *const Mb2MmapEntry) };

        // typ meanings (common):
        // 1 = available RAM, others reserved/ACPI/etc.
        let kind = match ent.entry_type {
            1 => "AVAILABLE",
            2 => "RESERVED",
            3 => "ACPI_RECLAIM",
            4 => "ACPI_NVS",
            5 => "BADRAM",
            _ => "OTHER",
        };

        serial_println!(
            "MB2: mmap[{:02}] base={:#016x} len={:#016x} type={} ({})",
            idx,
            ent.base_addr,
            ent.length,
            ent.entry_type,
            kind
        );

        idx += 1;
        e += mmap.entry_size as usize;
    }
}
