use alloc::vec::Vec;
use core::slice::Windows;
use ab_glyph_rasterizer::Rasterizer;
use spin::mutex::Mutex;
use spin::once::Once;
use crate::dwm::window::Window;
use crate::FONT_DATA;

pub static WM: Once<Mutex<WindowManager>> = Once::new();


pub fn init(width: usize, height: usize) {
    WM.call_once(|| {
        Mutex::new(WindowManager::new(width, height))
    });
}

pub fn add_window(window: Window) {
    if let Some(wm_mutex) = WM.get() {
        // 2. ロックを確保して
        let mut wm = wm_mutex.lock();
        // 3. リストに追加！
        wm.add_window(window);
    }
}


pub struct WindowManager {
    /// 管理しているウィンドウのリスト（奥から手前の順）
    pub windows: Vec<Window>,

    /// 画面の横幅（ピクセル）
    pub screen_width: usize,

    /// 画面の縦幅（ピクセル）
    pub screen_height: usize,

    /// 画面全体の作業用バッファ（下書きキャンバス）
    pub screen_buffer: Vec<u32>,
}

impl WindowManager {
    /// 指定された画面サイズでマネージャーを初期化する
    pub fn new(width: usize, height: usize) -> Self {
        // 画面の全ピクセル数を計算
        let total_pixels = width * height;

        Self {
            // 最初はウィンドウは一つもない
            windows: Vec::new(),
            screen_width: width,
            screen_height: height,
            // 画面全体のバッファを 0 (黒) で初期化
            // 13620Hのパワーなら、この巨大な Vec 確保も一瞬です
            screen_buffer: alloc::vec![0; total_pixels],
        }
    }

    pub fn add_window(&mut self, window: Window) {
        // Vec の push を使ってリストの最後に追加
        // リストの最後にあるものほど「手前」に描画されることになります
        self.windows.push(window);
    }

    fn is_in_screen(&self, x: i32, y: i32) -> bool {
        x >= 0 && (x as usize) < self.screen_width &&
            y >= 0 && (y as usize) < self.screen_height
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if self.is_in_screen(x, y) {
            let idx = (y as usize) * self.screen_width + (x as usize);
            self.screen_buffer[idx] = color;
        }
        // 画面外なら何もしない（クラッシュさせない）
    }


    pub fn compose(&mut self) {
        // --- STEP 1: 背景の塗りつぶし ---
        // まっさらな状態から描き始めます
        self.screen_buffer.fill(0x404040); // 落ち着いたダークグレー

        // --- STEP 2: 各ウィンドウを順番に描画 ---
        // インデックスで回すことで、self との借用衝突を回避します
        for i in 0..self.windows.len() {
            // 描画に必要なパラメータをコピー（借用時間を最小限にする）
            let (win_x, win_y, win_w, win_h) = {
                let win = &self.windows[i];
                (win.x, win.y, win.width, win.height)
            };

            // A. Windows 10風タイトルバーの強制描画 (高さ30px)
            let bar_h = 30;
            for row in 0..bar_h {
                for col in 0..win_w {
                    let sx = win_x + col as i32;
                    let sy = (win_y - bar_h as i32) + row as i32;

                    // 右端 30px は「×ボタン」の赤、それ以外は白
                    let color = if col > win_w - 30 { 0xE81123 } else { 0xF3F3F3 };
                    self.set_pixel(sx, sy, color);
                }
            }

            // B. ウィンドウ本体（アプリの中身）の描画
            // ここで一旦 buffer への参照を借りる
            let win_buffer = &self.windows[i].buffer;
            for row in 0..win_h {
                for col in 0..win_w {
                    // ここがコツ：一度色の値だけを取り出す
                    let color = self.windows[i].buffer[row * win_w + col];

                    // ここで self.set_pixel を呼ぶ
                    // 直前で color を「値」として取り出しているので、もう buffer を借りていない状態になります
                    self.set_pixel(win_x + col as i32, win_y + row as i32, color);
                }
            }
        }
    }

    pub fn flush(&self, vram_ptr: *mut u32) {
        // 合成用バッファの先頭ポインタ
        let src = self.screen_buffer.as_ptr();

        // 全ピクセル数
        let num_pixels = self.screen_width * self.screen_height;

        unsafe {
            // メモリを高速にコピー
            // 13620Hなら、4K解像度でも瞬きする間に終わります
            core::ptr::copy_nonoverlapping(src, vram_ptr, num_pixels);
        }
    }


}