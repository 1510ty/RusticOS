use limine::memory_map::{Entry, EntryType};
use crate::{CURRENT_Y, FRAMEBUFFER_BACK, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::drawstr::draw_str;
use crate::vga::{clear_back_buffer, request_update};

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
        // 1. スクロール（画面端）判定
        // SCREEN_HEIGHT も static mut にある想定
        if CURRENT_Y + 24 > SCREEN_HEIGHT as u64 {
            clear_back_buffer(0xFFFFFF); // さっき作った一括クリア
            CURRENT_Y = 0;
        }

        // 2. 描画実行
        // draw_str も内部で FRAMEBUFFER_BACK (static mut) を見るようにしてあれば
        // 引数はこれだけで済む
        draw_str(0, CURRENT_Y, s, 0x000000);

        // 3. 次の行へ
        CURRENT_Y += 24;

        request_update();
    }
}

pub fn print_usable_memory_stats(mmap: &[&Entry]) {

    let mut total_usable_bytes: u64 = 0;
    let mut usable_region_count: u64 = 0;

    for entry in mmap {
        // Usable（自由に使える）メモリだけを足していく
        if entry.entry_type == EntryType::USABLE {
            total_usable_bytes += entry.length;
            usable_region_count += 1;
        }
    }

    // バイトを MiB に変換 (1024 * 1024 = 1,048,576)
    let total_mib = total_usable_bytes / 1024 / 1024;

    println("--- Memory Stats ---");
    println("Usable regions found:");
    print_hex(usable_region_count); // 領域の数

    println("Total Usable Memory (MiB):");
    print_hex(total_mib); // 合計容量（16進数で出ちゃうけど今はOK！）
    println("--------------------");
}
