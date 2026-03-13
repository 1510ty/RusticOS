// kernel/dwm/window.rs

use alloc::vec;
use alloc::vec::Vec;

pub struct Window {
    pub x: i32,
    pub y: i32,
    pub width: usize,
    pub height: usize,
    // 13620H のメモリパワーを信じて Vec で確保
    pub buffer: Vec<u32>,
}

impl Window {
    pub fn new(x: i32, y: i32, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            width,
            height,
            // 指定サイズ分、真っ暗 (0x00000000) で初期化
            buffer: vec![0; width * height],
        }
    }

    /// ウィンドウ内の相対座標に描画する
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = color;
        }
    }
}