// kernel/dwm/manager.rs

use alloc::vec;
use alloc::vec::Vec;
use crate::dwm::window::Window;

pub struct WindowManager {
    pub windows: Vec<Window>,
    pub screen_width: usize,
    pub screen_height: usize,
    // 合成用の作業バッファ（VRAMに送る直前の「完成図」）
    pub screen_buffer: Vec<u32>,
}

impl WindowManager {
    /// 新しいマネージャーを作成
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            windows: Vec::new(),
            screen_width: width,
            screen_height: height,
            // 画面解像度分のバッファを確保
            screen_buffer: vec![0; width * height],
        }
    }

    /// ウィンドウを追加（後から追加したものが上に重なる）
    pub fn add_window(&mut self, win: Window) {
        self.windows.push(win);
    }

    /// すべてのウィンドウを合成して screen_buffer に書き込む
    pub fn compose(&mut self) {
        // まず背景をクリア（とりあえず黒）
        self.screen_buffer.fill(0xC0C0C0);

        for win in &self.windows {
            for row in 0..win.height {
                for col in 0..win.width {
                    let sx = win.x + col as i32;
                    let sy = win.y + row as i32;

                    // 画面の範囲内であれば合成
                    if sx >= 0 && (sx as usize) < self.screen_width &&
                        sy >= 0 && (sy as usize) < self.screen_height {
                        let color = win.buffer[row * win.width + col];

                        // ここで 0x000000 などを「透明」として扱えば透過ができる
                        if color != 0x000000 {
                            self.screen_buffer[sy as usize * self.screen_width + sx as usize] = color;
                        }
                    }
                }
            }
        }
    }

    /// 合成済みのバッファを本物の VRAM へ一気に転送
    pub fn flush_to_vram(&self, vram_ptr: *mut u32) {
        unsafe {
            // 13620H のパワーなら、core::ptr::copy_nonoverlapping で
            // メモリをガサッとコピーするのが最速です
            core::ptr::copy_nonoverlapping(
                self.screen_buffer.as_ptr(),
                vram_ptr,
                self.screen_width * self.screen_height
            );
        }
    }
}