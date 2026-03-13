use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::ptr::copy_nonoverlapping;
use spin::mutex::Mutex;
use crate::{HEIGHT, WIDTH, FRAMEBUFFER_BACK, FRAMEBUFFER_REQUEST, NEEDS_FRAME_UPDATE, SCREEN_HEIGHT, SCREEN_WIDTH, VRAM_PTR};
use crate::drawstr::draw_str;

fn swap_buffers() {
    unsafe {
        if let Some(ref back) = FRAMEBUFFER_BACK {
            // copy_nonoverlapping は Rust版 memcpy
            // 第3引数は「バイト数」ではなく「要素数(u32の数)」なのでこれでOK
            copy_nonoverlapping(
                back.as_ptr(),
                VRAM_PTR,
                SCREEN_WIDTH * SCREEN_HEIGHT
            );
        }
    }
}

// 画面を特定の色で塗りつぶす（リセット用）
pub fn clear_back_buffer(color: u32) {
    // 直接 fill せず、コマンドをキューの先頭（あるいは適切。な場所）に積む
    push_command(DrawCommand::Clear(color));

    // 「何か注文が入ったから、あとで画面更新してね」の合図だけ出す
    request_update();
}

pub fn request_update() {
    //swap_buffers();
    unsafe { NEEDS_FRAME_UPDATE = true; }
}


pub fn init_vga() {
    let fb_response = FRAMEBUFFER_REQUEST.get_response().unwrap();
    let fb = fb_response.framebuffers().next().expect("No framebuffer found");

    unsafe {
        VRAM_PTR = fb.addr() as *mut u32; // ここで「本物の住所」をメモ！
    }

    unsafe { WIDTH = fb.width() as usize;}
    unsafe { HEIGHT = fb.height() as usize;}

    unsafe {
        SCREEN_WIDTH = WIDTH;
        SCREEN_HEIGHT = HEIGHT;

        // ヒープから (幅 * 高さ * 4バイト) の領域を確保
        // これで Vec が裏画面の実体として固定される
        FRAMEBUFFER_BACK = Some(vec![0u32; WIDTH * HEIGHT]);

        // 念のため、最初は真っ黒（または好きな色）で塗りつぶしておく
        if let Some(ref mut back) = FRAMEBUFFER_BACK {
            back.fill(0x000000);
        }
    }
}

#[derive(Debug, Clone)]
pub enum DrawCommand {
    Text {
        x: usize,
        y: usize,
        color: u32,
        content: String,
    },
    Rect {
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        color: u32,
    },
    Clear(u32),
}

// Mutexで守られたグローバルなキュー
// これで unsafe な static mut を使わずに済みます
pub static DRAW_QUEUE: Mutex<Vec<DrawCommand>> = Mutex::new(Vec::new());

/// コマンドをキューに追加する
pub fn push_command(cmd: DrawCommand) {
    // lock()を呼ぶだけで安全にアクセス可能
    // 割り込み中でもロックが解除されるまで待機（スピン）します
    DRAW_QUEUE.lock().push(cmd);
}

/// キューの中身をすべて取り出し、現在のキューを空にする
pub fn fetch_commands() -> Vec<DrawCommand> {
    let mut queue = DRAW_QUEUE.lock();
    // 中身をまるごと入れ替えて古い方を返す（一瞬で終わる処理）
    core::mem::replace(&mut *queue, Vec::with_capacity(128))
}

/// キューが空かどうか判定する（描画の必要があるかチェックする用）
pub fn is_empty() -> bool {
    DRAW_QUEUE.lock().is_empty()
}

pub fn update_screen() {
    // 1. キューを全量回収（この間だけロック）
    let commands = fetch_commands();

    if commands.is_empty() {
        return;
    }

    // 2. 「俺様ルール」で描き込む
    for cmd in commands {
        match cmd {
            DrawCommand::Text { x, y, color, content } => {
                // ここで既存の文字描画ロジックを呼ぶ
                draw_str(x as u64, y as u64, &content, color);
            },
            DrawCommand::Rect { x, y, w, h, color } => {
                // ここで四角形描画
                // draw_rect(x, y, w, h, color);
            },
            DrawCommand::Clear(color) => {
                unsafe {
                    if let Some(ref mut back) = FRAMEBUFFER_BACK {
                        back.fill(color);
                    }
                }
            }
        }
    }

    // 3. 最後に一気にVRAMへ（ダブルバッファのフリップ）
    swap_buffers();
}