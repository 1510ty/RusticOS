use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use alloc::vec;
use alloc::vec::Vec;
use hashbrown::HashMap;

/// ラスタライズ済みの1文字データ
pub struct CachedGlyph {
    pub width: usize,
    pub height: usize,
    pub top: i32,
    pub left: i32,
    pub advance: f32,
    /// 0-255のアルファ値（不透明度）の配列
    pub alpha_map: Vec<u8>,
}

/// キャッシュの検索キー（文字とサイズ）
#[derive(Hash, Eq, PartialEq, Clone, Copy)]
struct GlyphKey {
    c: char,
    size_px: u32, // f32を10倍して整数化したものなど
}

pub struct FontManager<'a> {
    font: FontRef<'a>,
    cache: HashMap<GlyphKey, CachedGlyph>,
}

impl<'a> FontManager<'a> {
    pub fn new(font_data: &'a [u8]) -> Self {
        let font = FontRef::try_from_slice(font_data).expect("Failed to load font");
        Self {
            font,
            cache: HashMap::new(),
        }
    }

    /// 指定した文字とサイズのビットマップをキャッシュから（なければ生成して）返す
    pub fn get_glyph(&mut self, c: char, size: f32) -> &CachedGlyph {
        let key = GlyphKey {
            c,
            size_px: (size * 10.0) as u32,
        };

        // ★ 1. self.cache をいじる前に、必要な font への参照を「分離」して取り出す
        let font_ref = &self.font;

        self.cache.entry(key).or_insert_with(|| {
            // ★ 2. self ではなく、分離した font_ref を使う
            Self::rasterize_internal(font_ref, c, size)
        })
    }

    /// ★ 修正: self を使わない関連関数（static）にする
    fn rasterize_internal(font: &FontRef, c: char, size: f32) -> CachedGlyph {
        let scale = PxScale::from(size);
        let glyph = font.glyph_id(c).with_scale_and_position(scale, ab_glyph::point(0.0, 0.0));

        let h_advance = font.as_scaled(scale).h_advance(glyph.id);

        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            let width = bounds.width() as usize;
            let height = bounds.height() as usize;

            // pushだと順番がズレるリスクがあるので、サイズ指定で初期化が安全
            let mut alpha_map = vec![0u8; width * height];

            outlined.draw(|x, y, alpha| {
                if x < width as u32 && y < height as u32 {
                    alpha_map[(y as usize) * width + (x as usize)] = (alpha * 255.0) as u8;
                }
            });

            CachedGlyph {
                width,
                height,
                top: bounds.min.y as i32,
                left: bounds.min.x as i32,
                advance: h_advance,
                alpha_map,
            }
        } else {
            CachedGlyph {
                width: 0, height: 0, top: 0, left: 0,
                advance: h_advance,
                alpha_map: Vec::new(),
            }
        }
    }


    // ab_glyphを使ってベクターをビットマップに変換
    // fn rasterize_glyph(&self, c: char, size: f32) -> CachedGlyph {
    //     let scale = PxScale::from(size);
    //     let glyph = self.font.glyph_id(c).with_scale_and_position(scale, ab_glyph::point(0.0, 0.0));
    //
    //     // 1. 後で使う advance を先に計算して保存しておく
    //     let h_advance = self.font.as_scaled(scale).h_advance(glyph.id);
    //
    //     // 2. ここで glyph が消費（Move）される
    //     if let Some(outlined) = self.font.outline_glyph(glyph) {
    //         let bounds = outlined.px_bounds();
    //         let mut alpha_map = Vec::with_capacity((bounds.width() * bounds.height()) as usize);
    //
    //         outlined.draw(|_x, _y, alpha| {
    //             alpha_map.push((alpha * 255.0) as u8);
    //         });
    //
    //         CachedGlyph {
    //             width: bounds.width() as usize,
    //             height: bounds.height() as usize,
    //             top: bounds.min.y as i32,
    //             left: bounds.min.x as i32,
    //             advance: h_advance, // 先に取っておいた変数を使う
    //             alpha_map,
    //         }
    //     } else {
    //         CachedGlyph {
    //             width: 0, height: 0, top: 0, left: 0,
    //             advance: h_advance, // ここも同様
    //             alpha_map: Vec::new(),
    //         }
    //     }
    // }

}