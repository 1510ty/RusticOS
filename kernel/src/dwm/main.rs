use crate::*;


pub fn dwm_main(vram_ptr: *mut u32, width: usize, height: usize) -> ! {

    let mut manager = dwm::manager::WindowManager::new(width, height);
    //
    // let mut win1 = Window::new(10, 10, 200, 200, "Window1", true);
    // win1.fill(0xFFFFFF);
    // win1.draw_text("RusticOS",5,40,50.0,0x0,&mut manager.font_manager);
    // win1.draw_text("Ⓒ2026 1510ty",5, 60, 30.0 , 0, &mut manager.font_manager);
    //
    // let mut win2 = Window::new(10, 250, 400, 400, "Window2", true);
    // win2.fill(0xFFFFFF);
    // win2.draw_text("日本語もいけるよ!", 5,50,60.0,0,&mut manager.font_manager);
    // win2.draw_text("色もつけられる!", 5, 80, 40.0, 0x0000FF, &mut manager.font_manager);
    //
    // let mut win3 = Window::new(500, 30, 600, 330, "Window3", true);
    // win3.fill(0x0);
    //
    // win3.draw_text("起動にかかった時間: ", 5, 30, 30.0, 0xFFFFFF, &mut manager.font_manager);
    // win3.draw_text(get_uptime_ms().to_string().as_str(),200,30,30.0,0x00FF00,&mut manager.font_manager);
    // win3.draw_text("ms", 250, 30, 24.0, 0xFFFFFF, &mut manager.font_manager);
    //
    // win3.draw_text("CPU: ",5, 60, 30.0, 0xFFFFFF, &mut manager.font_manager);
    // win3.draw_text(arch::x86_64::getsysteminfo::get_cpu_name().as_str(),60,60,30.0,0x00FF00, &mut manager.font_manager);
    //
    //
    // manager.add_window(win1);
    // manager.add_window(win2);
    // manager.add_window(win3);
    //
    //
    //
    //
    // let mut loopcount: u64 = 0;

    xHCI::init_xhci(&mut manager);

    loop {
        // loopcount += 1;
        // if loopcount == 100000 {
        //     let mut win4 = Window::new(500, 500, 300, 300, "Window4", true);
        //     win4.fill(0x0000FF);
        //     win4.draw_text("このウィンドウは後から追加されました。", 5,50, 60.0, 0xFFFFFF, &mut manager.font_manager);
        //     manager.add_window(win4);
        // }
        manager.compose_all();
        manager.flush(vram_ptr);
    }
}
