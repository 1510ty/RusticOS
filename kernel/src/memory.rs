use limine::memory_map::EntryType;
use crate::{ALLOCATOR, MEMORY_MAP_REQUEST};

pub fn init_heap(hhdm_offset: u64) {
    let mmap_response = MEMORY_MAP_REQUEST.get_response().unwrap();
    let mmap = mmap_response.entries();

    // 2. 「一番大きい空き領域」をヒープとして使う
    // 実機は断片化しているので、最大の領域を探すのが安全
    let usable_entry = mmap.iter()
        .filter(|e| e.entry_type == EntryType::USABLE)
        .max_by_key(|e| e.length)
        .expect("No usable memory found for heap");

    // 3. 物理アドレスを HHDM 上の仮想アドレスに変換
    let heap_start = usable_entry.base + hhdm_offset;
    let heap_size = usable_entry.length;

    unsafe {
        // 4. アロケータを初期化（ここで Rust がメモリを使えるようになる）
        ALLOCATOR.lock().init(heap_start as *mut u8, heap_size as usize);
    }
}
