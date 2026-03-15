use crate::*;

pub fn init_xhci(manager: &mut dwm::manager::WindowManager) {
    let mut win = dwm::window::Window::new(400, 40, 500, 730, "xHCI Scanner", true);
    let f = &mut manager.font_manager;
    let hhdm_offset = unsafe { HHDM_OFFSET };

    let mut xhci_phys_base: u64 = 0;

    'scan: for bus in 0..256u32 {
        for slot in 0..32u32 {
            for func in 0..8u32 {
                let vendor_id = pci::pci_config_read_32(bus as u8, slot as u8, func as u8, 0x00) & 0xFFFF;
                if vendor_id == 0xFFFF { continue; }

                let class_rev = pci::pci_config_read_32(bus as u8, slot as u8, func as u8, 0x08);
                let class = (class_rev >> 24) & 0xFF;
                let sub = (class_rev >> 16) & 0xFF;
                let prog = (class_rev >> 8) & 0xFF;

                // xHCI Controller 発見
                if class == 0x0C && sub == 0x03 && prog == 0x30 {
                    win.draw_text("xHCI Found!", 20, 40, 16.0, 0x00FF00, f);

                    // --- [Step 1] BAR0/BAR1 の読み取りと住所の割り当て ---
                    let mut bar0 = pci::pci_config_read_32(bus as u8, slot as u8, func as u8, 0x10);
                    let mut bar1 = pci::pci_config_read_32(bus as u8, slot as u8, func as u8, 0x14);

                    // QEMU対策: BAR0の住所部分が空っぽ (0) の場合
                    if (bar0 & !0xF) == 0 {
                        win.draw_text("QEMU detected: Allocating BAR...", 20, 60, 14.0, 0xFFA500, f);

                        let target_addr: u64 = 0xFE00_0000;
                        // 下位32bitを書き込み (フラグ 0x4 等を保持)
                        pci::pci_config_write_32(bus as u8, slot as u8, func as u8, 0x10, (target_addr as u32) | (bar0 & 0xF));

                        // 64-bit BAR (Bit 2 が 1) なら上位32bit(BAR1)も書く
                        if (bar0 & 0x4) != 0 {
                            pci::pci_config_write_32(bus as u8, slot as u8, func as u8, 0x14, (target_addr >> 32) as u32);
                        }

                        // 書き込んだ後に再読込
                        bar0 = pci::pci_config_read_32(bus as u8, slot as u8, func as u8, 0x10);
                        bar1 = pci::pci_config_read_32(bus as u8, slot as u8, func as u8, 0x14);
                    }

                    // --- [Step 2] PCI Command レジスタを ON にする ---
                    let mut pci_cmd = pci::pci_config_read_32(bus as u8, slot as u8, func as u8, 0x04);
                    // Bit 1: Memory Space, Bit 2: Bus Master を有効化 (0x06)
                    pci_cmd |= 0x06;
                    pci::pci_config_write_32(bus as u8, slot as u8, func as u8, 0x04, pci_cmd);

                    // --- [Step 3] 最終的な物理アドレスの確定 ---
                    xhci_phys_base = if (bar0 & 0x4) != 0 {
                        ((bar1 as u64) << 32) | ((bar0 & !0xF) as u64)
                    } else {
                        (bar0 & !0xF) as u64
                    };

                    // 画面に結果を表示
                    for i in 0..2 {
                        let bar_offset = 0x10 + (i * 4);
                        let bar_val = pci::pci_config_read_32(bus as u8, slot as u8, func as u8, bar_offset as u8);
                        let y_pos = (100 + i * 25) as i32;
                        win.draw_text("BAR", 20, y_pos as usize, 14.0, 0xAAAAAA, f);
                        win.draw_hex(i as u32, 100, y_pos, f);
                        win.draw_hex(bar_val, 270, y_pos, f);
                    }

                    win.draw_text("Final Phys:", 20, 160, 14.0, 0xFFFF00, f);
                    win.draw_hex((xhci_phys_base >> 32) as u32, 150, 160, f); // 上位
                    win.draw_hex(xhci_phys_base as u32, 270, 160, f);       // 下位

                    break 'scan;
                }
            }
        }
    }

    // --- ここから先は実機・QEMU共通の「ページマッピング」フェーズへ ---
    if xhci_phys_base != 0 {
        // 例の PageTableManager でマップして、read_volatile する処理をここに書く
    }

    manager.add_window(win);
}