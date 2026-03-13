use alloc::vec::Vec;
use ab_glyph_rasterizer::{point, Rasterizer};
use ttf_parser::{Face, OutlineBuilder};
use crate::FONT_DATA;
use alloc::collections::BTreeMap;


// キャッシュに保存する1文字分のデータ
struct CachedGlyph {
    bitmap: Vec<u8>,
    width: usize,
    height: usize,
    advance: f32,
}

// WindowManagerか、あるいはグローバルなフォント管理者に持たせる
// FontCache の定義
pub struct FontCache {
    pub glyphs: BTreeMap<(char, u32), CachedGlyph>, // cache ではなく glyphs に統一
}

impl FontCache {
    pub fn new() -> Self {
        Self {
            glyphs: BTreeMap::new(), // ここも glyphs
        }
    }
}



pub struct Window {
    pub x: i32,          // 画面上のX座標
    pub y: i32,          // 画面上のY座標
    pub width: usize,    // ウィンドウの幅
    pub height: usize,   // ウィンドウの高さ
    pub buffer: Vec<u32>, // ウィンドウ専用の描画バッファ
}

impl Window {
    /// 新しいウィンドウ（キャンバス）を作成する
    pub fn new(x: i32, y: i32, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            width,
            height,
            // 指定されたサイズ分のメモリを確保
            buffer: alloc::vec![0; width * height],
        }
    }



    pub fn draw_char_with_cache(&mut self, cache: &mut FontCache, x: usize, y: usize, c: char, size: f32, color: u32) {
        let size_key = (size * 10.0) as u32; // 小数点誤差回避

        let canvas_width = (size * 1.2) as usize;
        let canvas_height = (size * 1.2) as usize;

        let mut rasterizer = Rasterizer::new(canvas_width, canvas_height);

        // 1. キャッシュをチェック
        if !cache.glyphs.contains_key(&(c, size_key)) {
            // --- キャッシュがないのでラスタライズ (以前の処理) ---
            let face = Face::parse(FONT_DATA, 0).unwrap();
            let glyph_id = face.glyph_index(c).unwrap();
            let scale = size / face.units_per_em() as f32;
            let advance = face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32 * scale;

            let mut rasterizer = Rasterizer::new(
                (size * 1.2) as usize, // 余裕を持たせたバッファサイズ
                (size * 1.2) as usize
            );

            let mut builder = GlyphPathBuilder {
                rasterizer: &mut rasterizer,
                scale,
                offset_x: 0.0,
                offset_y: size,
                last: point(0.0, 0.0),
            };
            face.outline_glyph(glyph_id, &mut builder);

            // ラスタライズ結果を Vec に取り出す
            let mut bitmap = Vec::new();
            rasterizer.for_each_pixel_2d(|_, _, alpha| {
                bitmap.push((alpha * 255.0) as u8);
            });

            cache.glyphs.insert((c, size_key), CachedGlyph {
                bitmap,
                width: canvas_width,  // rasterizer.width() の代わりにこれ
                height: canvas_height, // rasterizer.height() の代わりにこれ
                advance,
            });
        }

        // 2. キャッシュから取り出して画面に転送 (Blit)
        let glyph = cache.glyphs.get(&(c, size_key)).unwrap();
        self.blit_glyph(x, y, glyph, color);
    }

    // 高速なピクセル転送処理
    fn blit_glyph(&mut self, x: usize, y: usize, glyph: &CachedGlyph, color: u32) {
        for row in 0..glyph.height {
            for col in 0..glyph.width {
                let alpha = glyph.bitmap[row * glyph.width + col] as f32 / 255.0;
                if alpha > 0.0 {
                    let target_x = x + col;
                    let target_y = y + row;
                    if target_x < self.width && target_y < self.height {
                        let bg = self.buffer[target_y * self.width + target_x];
                        self.buffer[target_y * self.width + target_x] = blend_color(bg, color, alpha);
                    }
                }
            }
        }
    }

    pub fn draw_vector_str_cached(
        &mut self,
        cache: &mut FontCache,
        x: usize,
        y: usize,
        s: &str,
        size: f32,
        color: u32
    ) {
        let face = ttf_parser::Face::parse(FONT_DATA, 0).unwrap();
        let scale = size / face.units_per_em() as f32;
        let mut current_x = x as f32;

        for c in s.chars() {
            if let Some(glyph_id) = face.glyph_index(c) {
                // 1. キャッシュを利用して描画（なければ自動生成される）
                self.draw_char_with_cache(cache, current_x as usize, y, c, size, color);

                // 2. カーニング（送り幅）の取得
                let advance = face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32 * scale;
                current_x += advance;
            }
        }
    }


}

struct GlyphPathBuilder<'a> {
    rasterizer: &'a mut Rasterizer,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
    last: ab_glyph_rasterizer::Point, // 現在の座標を保持
}

impl OutlineBuilder for GlyphPathBuilder<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        // 現在地を更新するだけ
        self.last = point(x * self.scale + self.offset_x, -y * self.scale + self.offset_y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let to = point(x * self.scale + self.offset_x, -y * self.scale + self.offset_y);
        // last から to へ直線を引く
        self.rasterizer.draw_line(self.last, to);
        self.last = to;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let control = point(x1 * self.scale + self.offset_x, -y1 * self.scale + self.offset_y);
        let to = point(x * self.scale + self.offset_x, -y * self.scale + self.offset_y);
        // last から to へ、control を制御点とした2次ベジェ曲線を引く
        self.rasterizer.draw_quad(self.last, control, to);
        self.last = to;
    }

    fn curve_to(&mut self, _x1: f32, _y1: f32, _x2: f32, _y2: f32, _x: f32, _y: f32) {
        // TrueType(Noto Sans)は使いませんが、一応 line_to で近似するか無視
    }

    fn close(&mut self) {
        // 必要なら開始点に戻る線を引くが、通常は rasterizer が処理する
    }
}

fn blend_color(back: u32, fore: u32, alpha: f32) -> u32 {
    let a = (alpha * 255.0) as u32;
    let r = (((fore >> 16) & 0xff) * a + ((back >> 16) & 0xff) * (255 - a)) / 255;
    let g = (((fore >> 8) & 0xff) * a + ((back >> 8) & 0xff) * (255 - a)) / 255;
    let b = ((fore & 0xff) * a + (back & 0xff) * (255 - a)) / 255;
    (r << 16) | (g << 8) | b
}