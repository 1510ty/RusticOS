use alloc::vec;
use alloc::vec::Vec;
use crate::dwm::font::FontManager;
use crate::dwm::window::Window;
use crate::FONT_DATA;

pub struct WindowManager<'a> {
    pub screen_width: usize,
    pub screen_height: usize,
    pub windows: Vec<Window>,      // 下から順に格納（最後が最前面）
    pub back_buffer: Vec<u32>,     // 画面全体と同じサイズの描画用メモリ
    // pub mouse_x: isize,
    // pub mouse_y: isize,
    pub font_manager: FontManager<'a>,
}

impl WindowManager<'_> {


    pub fn new(width: usize, height: usize) -> Self {
        Self {
            screen_width: width,
            screen_height: height,
            windows: Vec::new(),
            // 13620Hなら数MBの確保も一瞬！
            back_buffer: vec![0x008080; width * height], // 初期色は懐かしのグリーン
            // mouse_x: 0,
            // mouse_y: 0,
            font_manager: FontManager::new(FONT_DATA),
        }
    }

    /// 新しいウィンドウをリストに追加（一番手前に来る）
    pub fn add_window(&mut self, window: Window) {
        self.windows.push(window);
    }

    /// すべてのウィンドウをバックバッファに書き込む
    pub fn compose_all(&mut self) {


        // 1. 背景でリセット
        self.back_buffer.fill(0x999999);

        // 2. 下にあるウィンドウから順番に「重ね塗り」
        for win in &self.windows {
            if !win.is_visible { continue; }

            // 1. タイトルバーの高さを決定
            let title_height = if win.has_title_bar { 24 } else { 0 };

            // --- A. タイトルバー自体の描画 (OSの仕事) ---
            if win.has_title_bar {
                for ty in 0..title_height {
                    let screen_y = win.y + ty as isize;
                    if screen_y < 0 || screen_y >= self.screen_height as isize { continue; }

                    for tx in 0..win.width {
                        let screen_x = win.x + tx as isize;
                        if screen_x < 0 || screen_x >= self.screen_width as isize { continue; }

                        // アクティブなら青、そうでなければグレーなど
                        let color = if win.is_active { 0x0000AA } else { 0x555555 };
                        self.back_buffer[screen_y as usize * self.screen_width + screen_x as usize] = color;
                    }
                }
                //  ここで文字(win.title)を描画する関数を呼ぶ
            }

            // --- B. ウィンドウ本体(buffer)の描画 ---
            for y in 0..win.height {
                // 重要：タイトルバーの分だけ下にずらす
                let screen_y = win.y + (y + title_height) as isize;
                if screen_y < 0 || screen_y >= self.screen_height as isize { continue; }

                for x in 0..win.width {
                    let screen_x = win.x + x as isize;
                    if screen_x < 0 || screen_x >= self.screen_width as isize { continue; }

                    let color = win.buffer[y * win.width + x];
                    // 透明度(Alpha)をやるならここで計算。13620Hなら余裕！
                    self.back_buffer[screen_y as usize * self.screen_width + screen_x as usize] = color;
                }
            }
        }
    }

    pub fn flush(&self, vram_ptr: *mut u32) {
        // 13620H なら copy_from_slice が爆速！
        // 安全のために、書き込み先をスライスとして扱う
        let vram_slice = unsafe {
            core::slice::from_raw_parts_mut(vram_ptr, self.screen_width * self.screen_height)
        };

        // メモリのコピー（これが一番早い）
        vram_slice.copy_from_slice(&self.back_buffer);
    }

}