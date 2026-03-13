// kernel/dwm/main.rs
use crate::dwm::manager::WindowManager;
use crate::dwm::window::Window;

/// DWMスレッドのエントリーポイント
// kernel/dwm/main.rs

pub fn dwm_main(vram_ptr: *mut u32, width: usize, height: usize) -> ! {
    let mut wm = WindowManager::new(width, height);

    // 最初の一歩：テスト用の緑窓を (100, 100) に置いてみる
    let mut test_win = Window::new(100, 100, 200, 200);
    test_win.buffer.fill(0x00FF00); // 緑!!!
    wm.add_window(test_win);



    loop {
        wm.compose();
        wm.flush_to_vram(vram_ptr);

        // ここで少しだけ CPU を休ませると、他のスレッドが動きやすくなります
        core::hint::spin_loop();
    }
}