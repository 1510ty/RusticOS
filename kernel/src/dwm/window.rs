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

    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x]
        } else {
            0 // 範囲外はとりあえず黒（または透過）
        }
    }

    pub fn draw_glyph(&mut self, start_x: usize, start_y: usize, color: u32, glyph: &crate::dwm::font::CachedGlyph) {
        for dy in 0..glyph.height {
            for dx in 0..glyph.width {
                let px = start_x + dx;
                let py = start_y + dy;

                // alpha値を取得 (0-255)
                let alpha = glyph.alpha_map[dy * glyph.width + dx];
                if alpha == 0 { continue; }

                if alpha == 255 {
                    self.set_pixel(px, py, color);
                } else {
                    // アルファブレンディング
                    let bg = self.get_pixel(px, py);
                    let blended = self.blend(bg, color, alpha);
                    self.set_pixel(px, py, blended);
                }
            }
        }
    }

    /// RGBブレンディング計算 (13620Hなら整数演算で十分高速)
    fn blend(&self, bg: u32, fg: u32, alpha: u8) -> u32 {
        let a = alpha as u32;
        let inv_a = 255 - a;

        let r = (((fg >> 16) & 0xff) * a + ((bg >> 16) & 0xff) * inv_a) / 255;
        let g = (((fg >> 8) & 0xff) * a + ((bg >> 8) & 0xff) * inv_a) / 255;
        let b = ((fg & 0xff) * a + (bg & 0xff) * inv_a) / 255;

        (r << 16) | (g << 8) | b
    }

    pub fn draw_text(
        &mut self,
        text: &str,
        x: usize,
        y: usize,
        size: f32,
        color: u32,
        font_manager: &mut crate::dwm::font::FontManager,
    ) {
        let mut cursor_x = x as f32;
        // ベースラインの調整用にフォントの基本情報を取得（任意で調整）
        // とりあえず y をベースライン（文字の底辺付近）として扱います

        for c in text.chars() {
            // 1. キャッシュからグリフ情報を取得
            let glyph = font_manager.get_glyph(c, size);

            // 2. 描画位置の計算
            // glyph.top はベースラインからのオフセット（通常は負の値）
            let draw_x = (cursor_x + glyph.left as f32) as usize;
            let draw_y = (y as i32 + glyph.top) as usize;

            // 3. 1文字描画
            self.draw_glyph(draw_x, draw_y, color, &*glyph);

            // 4. カーソルを次に進める
            cursor_x += glyph.advance;
        }
    }

    pub fn write_text(
        &mut self,
        text: &str,
        x: usize,
        y: usize,
        size: f32,
        color: u32,
        font_manager: &mut crate::dwm::font::FontManager
    ) {
        // タイトルバーがある場合、コンテンツの開始位置を y 分オフセットする
        let offset_y = if self.has_title_bar { 30 } else { 0 };

        // 実際の描画処理（先に作った draw_text を呼ぶ）
        self.draw_text(text, x, y + offset_y, size, color, font_manager);
    }

}