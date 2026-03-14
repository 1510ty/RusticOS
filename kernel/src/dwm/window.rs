use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

pub struct Window {
    // --- 基本位置・サイズ ---
    pub x: isize,
    pub y: isize,
    pub width: usize,
    pub height: usize,

    // --- 表示設定（自分好みポイント） ---
    pub title: String,
    pub is_visible: bool,       // 非表示（最小化）フラグ
    pub has_title_bar: bool,    // タイトルバーなし（ポップアップ用）
    pub can_resizable: bool,     // リサイズ可能か
    pub can_movable: bool,       // ドラッグで動かせるか

    pub(crate) is_active: bool,

    // --- データバッファ ---
    pub buffer: Vec<u32>,

}

impl Window {
    /// 新しいウィンドウを生成する
    pub fn new(
        x: isize,
        y: isize,
        width: usize,
        height: usize,
        title: &str,
        has_title_bar: bool,
    ) -> Self {
        let buffer_size = width * height;
        // 最初は透明（あるいは黒）で塗りつぶしておく
        let buffer = vec![0x000000; buffer_size];

        Self {
            x,
            y,
            width,
            height,
            title: title.to_string(),
            is_visible: true,
            has_title_bar,
            can_resizable: true, // デフォルト設定
            can_movable: true,
            is_active: false,
            buffer,
        }
    }

    /// ウィンドウ内の指定座標に色を置く
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = color;
        }
    }

    /// ウィンドウ全体を指定した色で塗りつぶす
    pub fn fill(&mut self, color: u32) {
        self.buffer.fill(color);
    }

    /// ウィンドウ内の矩形塗りつぶし
    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for dy in 0..h {
            let py = y + dy;
            if py >= self.height { break; }
            for dx in 0..w {
                let px = x + dx;
                if px >= self.width { break; }
                self.buffer[py * self.width + px] = color;
            }
        }
    }

}