use core::sync::atomic::Ordering;
use crate::vga::{push_command, request_update, update_screen, DrawCommand};
use crate::{CURRENT_Y, FRAMEBUFFER_BACK, INITIALIZED, SCREEN_HEIGHT, SCREEN_WIDTH};
use limine::memory_map::{Entry, EntryType};

pub fn draw_glyph(x_start: u64, y_start: u64, glyph: &[u32; 24]) {
    unsafe {
        if let Some(ref mut buffer) = FRAMEBUFFER_BACK {
            for row in 0..24 {
                let row_data = glyph[row];
                for col in 0..24 {
                    if (row_data >> (31 - col)) & 1 == 1 {
                        // バックバッファ上の座標計算
                        // (y * 幅 + x)
                        let offset = (y_start + row as u64) * SCREEN_WIDTH as u64 + (x_start + col as u64);

                        // バッファの境界チェック（念のため）
                        if offset < buffer.len() as u64 {
                            buffer[offset as usize] = 0x000000;
                        }
                    }
                }
            }
        }
    }
}

pub fn print_hex(value: u64) {
    let mut buf = [0u8; 18];
    let hex_chars = b"0123456789ABCDEF";
    buf[0] = b'0';
    buf[1] = b'x';
    for i in 0..16 {
        buf[i + 2] = hex_chars[((value >> ((15 - i) * 4)) & 0xF) as usize];
    }

    if let Ok(s) = core::str::from_utf8(&buf) {
        // ここで自慢の println を呼ぶ！
        // これで座標管理もロックも全部おまかせ。
        println(s);
    }
}


pub fn println(s: &str) {
    unsafe {
        // 1. スクロール判定（ここは座標管理なのでそのままでOK）
        if CURRENT_Y + 24 > SCREEN_HEIGHT as u64 {
            // 直接消すのではなく「画面クリア」というコマンドを送る
            push_command(DrawCommand::Clear(0xFFFFFF));
            CURRENT_Y = 0;
        }

        // 2. 描画「予約」
        // 直接 draw_str を呼ばず、キューに積む！
        push_command(DrawCommand::Text {
            x: 0,
            y: CURRENT_Y as usize,
            color: 0x000000,
            content: alloc::string::String::from(s), // 所有権を渡す
        });

        // 3. 次の行へ
        CURRENT_Y += 24;

        // request_update() は「描画が必要だよ」というフラグ立てとして残す
        if INITIALIZED.load(Ordering::Relaxed) == false {
            //起動が完了していない場合
            update_screen();
        } else {
            request_update();
        }

    }
}
