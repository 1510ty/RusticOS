use alloc::vec;
use core::ptr::copy_nonoverlapping;
use crate::{FRAMEBUFFER_BACK, FRAMEBUFFER_REQUEST, SCREEN_HEIGHT, SCREEN_WIDTH};

static mut VRAM_PTR: *mut u32 = core::ptr::null_mut();

pub fn swap_buffers(fb_ptr: *mut u32) {
    unsafe {
        if let Some(ref back) = FRAMEBUFFER_BACK {
            // copy_nonoverlapping は Rust版 memcpy
            // 第3引数は「バイト数」ではなく「要素数(u32の数)」なのでこれでOK
            copy_nonoverlapping(
                back.as_ptr(),
                fb_ptr,
                SCREEN_WIDTH * SCREEN_HEIGHT
            );
        }
    }
}

// 画面を特定の色で塗りつぶす（リセット用）
pub fn clear_back_buffer(color: u32) {
    unsafe {
        if let Some(ref mut back) = FRAMEBUFFER_BACK {
            // slice::fill はループを回すより圧倒的に速い
            back.fill(color);
            request_update();
        }
    }
}

pub fn request_update() {
    unsafe {swap_buffers(VRAM_PTR);}
    //unsafe { NEEDS_FRAME_UPDATE = true; }
}


pub fn init_vga() {
    let fb_response = FRAMEBUFFER_REQUEST.get_response().unwrap();
    let fb = fb_response.framebuffers().next().expect("No framebuffer found");

    unsafe {
        VRAM_PTR = fb.addr() as *mut u32; // ここで「本物の住所」をメモ！
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    unsafe {
        SCREEN_WIDTH = width;
        SCREEN_HEIGHT = height;

        // ヒープから (幅 * 高さ * 4バイト) の領域を確保
        // これで Vec が裏画面の実体として固定される
        FRAMEBUFFER_BACK = Some(vec![0u32; width * height]);

        // 念のため、最初は真っ黒（または好きな色）で塗りつぶしておく
        if let Some(ref mut back) = FRAMEBUFFER_BACK {
            back.fill(0x000000);
        }
    }
}
