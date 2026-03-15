use crate::*;

pub fn init_xhci(manager: &mut dwm::manager::WindowManager) {
    let mut win = dwm::window::Window::new(400, 40, 500, 730, "xHCI Scanner", true);

    let f = &mut manager.font_manager;

    let hhdm_offset = unsafe { HHDM_OFFSET };



    // --- Step 1: PCI Scan & MMIO Setup ---

    let mut xhci_mmio_base: u64 = 0;



    'scan: for bus in 0..256u32 {
        for slot in 0..32u32 {
            for func in 0..8u32 {
                // 関数に渡す時だけ u8 にキャスト
                let vendor_id = pci::pci_config_read_32(bus as u8, slot as u8, func as u8, 0x00) & 0xFFFF;
                if vendor_id == 0xFFFF { continue; }

                let class_rev = pci::pci_config_read_32(bus as u8, slot as u8, func as u8, 0x08);
                let class = (class_rev >> 24) & 0xFF;
                let sub = (class_rev >> 16) & 0xFF;
                let prog = (class_rev >> 8) & 0xFF;

                if class == 0x0C && sub == 0x03 && prog == 0x30 {
                    win.draw_text("xHCI Found!", 20, 40, 16.0, 0x00FF00, f);

                    // i を u32 で回すと計算が楽です
                    for i in 0..6u32 {
                        let bar_offset = 0x10 + (i * 4);
                        let bar_val = pci::pci_config_read_32(bus as u8, slot as u8, func as u8, bar_offset as u8);

                        let y_pos = (80 + i * 25) as i32; // 行間を少し広げました
                        win.draw_text("BAR", 20, y_pos as usize, 14.0, 0xAAAAAA, f);
                        win.draw_hex(i as u64 as u32, 60, y_pos, f);
                        win.draw_hex(bar_val as u64 as u32, 120, y_pos, f);
                    }

                    // PCI Commandレジスタの確認
                    let pci_cmd = pci::pci_config_read_32(bus as u8, slot as u8, func as u8, 0x04);
                    win.draw_text("PCI CMD:", 20, 240, 14.0, 0xFFFF00, f);
                    win.draw_hex(pci_cmd as u64 as u32, 120, 240, f);

                    // これで画面が止まって見えるはず
                    break 'scan;
                }
            }
        }
    }

    let mmap_response = MEMORY_MAP_REQUEST.get_response().unwrap();
    let mmap = mmap_response.entries();

    // 全エントリの中で、最も高い物理アドレスを探す
    let last_entry = mmap.iter()
        .max_by_key(|e| e.base + e.length)
        .unwrap();

    let max_phys = last_entry.base + last_entry.length;

    win.draw_hex(max_phys as u32, 20, 300, f);

    manager.add_window(win);
    return;

    if xhci_mmio_base != 0 {

        // Capability Register の最初の4バイトを読み取る

        // 下位8ビットが CAPLENGTH (Operational Regs までのオフセット)

        let cap_reg = unsafe { core::ptr::read_volatile(xhci_mmio_base as *const u32) };


        win.draw_text("Cap Reg (HCIVERSION):", 20, 80, 14.0, 0xFFFFFF, f);

        win.draw_hex(cap_reg as u64 as u32, 200, 80, f);



        // QEMUなら大抵 0x01000020 (CAPLENGTH=0x20, HCIVERSION=0x0100) が出ます

    }


}