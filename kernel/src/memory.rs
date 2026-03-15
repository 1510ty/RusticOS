use limine::memory_map::EntryType;
use crate::{ALLOCATOR, MEMORY_MAP_REQUEST};

// --- 1. 構造体定義 ---

#[repr(align(4096))]
pub struct PageTable {
    pub entries: [u64; 512],
}

impl PageTable {
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = 0;
        }
    }
}

pub struct PageTableManager {
    pub pml4: &'static mut PageTable,
    pub hhdm_offset: u64,
}

// --- 2. 物理フレーム管理 (ページテーブル専用) ---

static mut FRAME_CURSOR: u64 = 0;
static mut FRAME_END: u64 = 0;

/// 4KBの物理ページを1枚払い出す
fn allocate_frame() -> u64 {
    unsafe {
        if FRAME_CURSOR + 4096 > FRAME_END {
            panic!("Out of frames for page tables!");
        }
        let frame = FRAME_CURSOR;
        FRAME_CURSOR += 4096;
        frame
    }
}

// --- 3. メモリ初期化 ---

pub fn init_memory(hhdm_offset: u64) {
    let mmap_response = MEMORY_MAP_REQUEST.get_response().unwrap();
    let mmap = mmap_response.entries();

    // 一番大きい領域を探す
    let usable_entry = mmap.iter()
        .filter(|e| e.entry_type == EntryType::USABLE)
        .max_by_key(|e| e.length)
        .expect("No usable memory found");

    // 末尾1MBをページテーブル資材用に予約
    let pt_area_size = 1024 * 1024;
    unsafe {
        FRAME_CURSOR = usable_entry.base + usable_entry.length - pt_area_size;
        FRAME_END = usable_entry.base + usable_entry.length;
    }

    // 残りの領域をヒープ（Rustアロケータ）に渡す
    let heap_start = usable_entry.base + hhdm_offset;
    let heap_size = usable_entry.length - pt_area_size;
    unsafe {
        ALLOCATOR.lock().init(heap_start as *mut u8, heap_size as usize);
    }
}

// --- 4. ページテーブル操作 ---

impl PageTableManager {
    pub fn new(pml4: &'static mut PageTable, hhdm_offset: u64) -> Self {
        Self { pml4, hhdm_offset }
    }

    pub unsafe fn map_page(&mut self, virt: u64, phys: u64, flags: u64) {
        let pml4_idx = ((virt >> 39) & 0x1FF) as usize;
        let pdpt_idx = ((virt >> 30) & 0x1FF) as usize;
        let pd_idx   = ((virt >> 21) & 0x1FF) as usize;
        let pt_idx   = ((virt >> 12) & 0x1FF) as usize;

        // self を経由せず、関数として呼び出す
        let pdpt = get_or_create_table(&mut self.pml4.entries[pml4_idx], self.hhdm_offset);
        let pd = get_or_create_table(&mut pdpt.entries[pdpt_idx], self.hhdm_offset);
        let pt = get_or_create_table(&mut pd.entries[pd_idx], self.hhdm_offset);

        pt.entries[pt_idx] = phys | flags;

        core::arch::asm!("invlpg [{}]", in(reg) virt);
    }
}

/// impl の外に出して、ただの関数にする
unsafe fn get_or_create_table(entry: &mut u64, hhdm_offset: u64) -> &'static mut PageTable {
    if (*entry & 1) == 0 {
        let new_frame_phys = allocate_frame();
        let new_table_virt = (new_frame_phys + hhdm_offset) as *mut PageTable;
        (*new_table_virt).clear();

        *entry = new_frame_phys | 0x007;
    }

    let phys = *entry & 0x000F_FFFF_FFFF_F000;
    &mut *((phys + hhdm_offset) as *mut PageTable)
}

/// 現在稼働中のPML4を取得する
pub fn get_current_pml4(hhdm_offset: u64) -> &'static mut PageTable {
    let cr3: u64;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
    }
    let pml4_phys = cr3 & 0x000F_FFFF_FFFF_F000;
    unsafe { &mut *((pml4_phys + hhdm_offset) as *mut PageTable) }
}