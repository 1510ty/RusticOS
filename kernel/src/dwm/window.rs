use crate::dwm::font::{draw_vector_str_generic, FontCache};
use alloc::string::String;
use alloc::vec::Vec;


pub struct Window {
    pub x: i32,          // 画面上のX座標
    pub y: i32,          // 画面上のY座標
    pub width: usize,    // ウィンドウの幅
    pub height: usize,   // ウィンドウの高さ
    pub buffer: Vec<u32>, // ウィンドウ専用の描画バッファ
    pub title: String,
}



pub fn draw_window_text(win: &mut Window, cache: &mut FontCache, x: usize, y: usize, text: &str, size: f32, color: u32) {
    draw_vector_str_generic(&mut win.buffer, win.width, win.height, cache, x, y, text, size, color);
}


pub fn blend_color(back: u32, fore: u32, alpha: f32) -> u32 {
    let a = (alpha * 255.0) as u32;
    let r = (((fore >> 16) & 0xff) * a + ((back >> 16) & 0xff) * (255 - a)) / 255;
    let g = (((fore >> 8) & 0xff) * a + ((back >> 8) & 0xff) * (255 - a)) / 255;
    let b = ((fore & 0xff) * a + (back & 0xff) * (255 - a)) / 255;
    (r << 16) | (g << 8) | b
}

