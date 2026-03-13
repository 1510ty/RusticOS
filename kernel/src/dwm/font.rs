use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use ab_glyph_rasterizer::{point, Rasterizer};
use ttf_parser::{Face, OutlineBuilder};
use crate::dwm::window::{blend_color};
use crate::FONT_DATA;


pub struct CachedGlyph {
    pub(crate) bitmap: Vec<u8>,
    pub(crate) width: usize,
    pub(crate) height: usize,
    advance: f32,
}

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


pub struct GlyphPathBuilder<'a> {
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



/// 描画対象のバッファ、幅、高さ、キャッシュ、そして描画情報を渡せばどこにでも描ける関数
pub fn draw_vector_str_generic(
    buffer: &mut [u32],
    buf_width: usize,
    buf_height: usize,
    cache: &mut FontCache,
    x: usize,
    y: usize,
    s: &str,
    size: f32,
    color: u32,
) {
    let face = Face::parse(FONT_DATA, 0).unwrap();
    let scale = size / face.units_per_em() as f32;
    let mut current_x = x as f32;

    for c in s.chars() {
        if let Some(glyph_id) = face.glyph_index(c) {
            // キャッシュ取得（なければ生成）
            let glyph = get_or_create_glyph(cache, c, size);

            // 描画（Blit）
            blit_glyph_generic(buffer, buf_width, buf_height, current_x as usize, y, glyph, color);

            let advance = face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32 * scale;
            current_x += advance;
        }
    }
}

/// キャッシュの管理だけを行う関数
fn get_or_create_glyph(cache: &mut FontCache, c: char, size: f32) -> &CachedGlyph {
    let size_key = (size * 10.0) as u32;

    if !cache.glyphs.contains_key(&(c, size_key)) {
        // --- ここに以前のラスタライズ処理を移動 ---
        // (省略しますが、中身は以前の draw_char_with_cache の生成部分です)
    }
    cache.glyphs.get(&(c, size_key)).unwrap()
}

/// バッファにピクセルを転送するだけの関数
fn blit_glyph_generic(buffer: &mut [u32], buf_width: usize, buf_height: usize, x: usize, y: usize, glyph: &CachedGlyph, color: u32) {
    for row in 0..glyph.height {
        for col in 0..glyph.width {
            let alpha = glyph.bitmap[row * glyph.width + col] as f32 / 255.0;
            if alpha > 0.0 {
                let (tx, ty) = (x + col, y + row);
                if tx < buf_width && ty < buf_height {
                    let idx = ty * buf_width + tx;
                    buffer[idx] = blend_color(buffer[idx], color, alpha);
                }
            }
        }
    }
}