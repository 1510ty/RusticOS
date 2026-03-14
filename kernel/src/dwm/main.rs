use crate::dwm::manager::WindowManager;
use crate::dwm::window::Window;

pub fn dwm_main(vram_ptr: *mut u32, width: usize, height: usize) -> ! {


    let mut manager = WindowManager::new(width, height);

    let mut my_app_window = Window::new(
        100, 100,    // x, y
        400, 300,    // width, height
        "My Cool App", // タイトル
        true          // OSのタイトルバーを付ける
    );

    my_app_window.fill(0xFFFFFF);
    my_app_window.fill_rect(50, 50, 100, 100, 0xFF0000);

    manager.add_window(my_app_window);


    loop {
        manager.compose_all();
        manager.flush(vram_ptr);

        core::hint::spin_loop();
    }
}