use x86_64::instructions::hlt;
use crate::dwm::manager;
use crate::dwm::manager::WM;
use crate::dwm::window::{FontCache, Window};
use crate::HEIGHT;

/// DWMスレッドのエントリーポイント
// kernel/dwm/main.rs

pub fn dwm_main(vram_ptr: *mut u32, width: usize, height: usize) -> ! {

    manager::init(width, height);

    let mut pci_win = Window::new(50, 50, 600, 400);
    pci_win.buffer.fill(0x222222); // 背景色

    // タイトルを表示 (サイズ 32.0)

    let mut global_font_cache = FontCache::new();

    pci_win.draw_vector_str_cached(&mut global_font_cache,20, 40, "PCI Device Manager", 32.0, 0x00FF00);

    // デバイス情報を表示 (サイズ 20.0)
    // ※ stdがない場合、format! の代わりに自作の文字列変換を使う必要があります
    pci_win.draw_vector_str_cached(&mut global_font_cache,20, 80, "Bus 00 Dev 02: Intel Graphics", 20.0, 0xFFFFFF);
    pci_win.draw_vector_str_cached(&mut global_font_cache,20, 110, "Bus 00 Dev 1f: Intel LPC Controller", 20.0, 0xFFFFFF);

    manager::add_window(pci_win);



    loop {
        if let Some(wm_mutex) = WM.get() {
            let mut wm = wm_mutex.lock();

            // 2. 合成（下書きバッファを完成させる）
            wm.compose();

            // 3. 転送（VRAMへ一気にコピー）
            wm.flush(vram_ptr);
        }

        // 4. 少し休憩（CPU 100% 張り付きを防止）
        // ※ OSに sleep 命令があれば入れる。なければ x86 の pause 命令
        core::hint::spin_loop();
    }
}