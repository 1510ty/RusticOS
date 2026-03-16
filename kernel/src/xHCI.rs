use core::hint::spin_loop;
use crate::*;

pub fn init_xhci(manager: &mut dwm::manager::WindowManager) {
    let mut win = dwm::window::Window::new(200, 40, 800, 730, "xHCI Scanner", true);
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
                    // win.draw_text("xHCI Found!", 20, 40, 16.0, 0x00FF00, f);

                    // --- [Step 1] BAR0/BAR1 の読み取りと住所の割り当て ---
                    let mut bar0 = pci::pci_config_read_32(bus as u8, slot as u8, func as u8, 0x10);
                    let mut bar1 = pci::pci_config_read_32(bus as u8, slot as u8, func as u8, 0x14);

                    // QEMU対策: BAR0の住所部分が空っぽ (0) の場合
                    if (bar0 & !0xF) == 0 {
                        // win.draw_text("QEMU detected: Allocating BAR...", 20, 60, 14.0, 0xFFA500, f);

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
                        // win.draw_text("BAR", 20, y_pos as usize, 14.0, 0xAAAAAA, f);
                        // win.draw_hex(i as u32, 100, y_pos, f);
                        // win.draw_hex(bar_val, 270, y_pos, f);
                    }

                    // win.draw_text("Final Phys:", 20, 160, 14.0, 0xFFFF00, f);
                    // win.draw_hex((xhci_phys_base >> 32) as u32, 150, 160, f); // 上位
                    // win.draw_hex(xhci_phys_base as u32, 270, 160, f);       // 下位

                    break 'scan;
                }
            }
        }
    }

    // --- ここから先は実機・QEMU共通の「ページマッピング」フェーズへ ---
    if xhci_phys_base != 0 {
        // --- 1. ページテーブルの準備 ---
        let pml4 = unsafe { memory::get_current_pml4(hhdm_offset) };
        let mut page_manager = memory::PageTableManager::new(pml4, hhdm_offset);

        // 仮想アドレスはカーネル空間の空いている場所（例：0xffff_a000_0000_0000）を使用
        let xhci_virt: u64 = 0xffff_a000_0000_0000;

        // マップ実行：Present(1) | Writable(2) | Write-through(8) | Cache Disable(16) = 0x1B
        // MMIOなのでキャッシュを無効化するのが鉄則です
        unsafe {
            page_manager.map_page(xhci_virt, xhci_phys_base, 0x1B);
        }

        // win.draw_text("Mapping Successful!", 20, 200, 14.0, 0x00FF00, f);

        // --- 2. レジスタの読み取り ---
        // 先頭 4 バイトを読み取る
        // [0:7]   CAPLENGTH (Capability Register Length)
        // [8:15]  Reserved
        // [16:31] HCIVERSION (Interface Version Number)
        let cap_reg = unsafe { core::ptr::read_volatile(xhci_virt as *const u32) };

        let cap_length = (cap_reg & 0xFF) as u8;
        let hci_version = (cap_reg >> 16) as u16;

        // --- 3. 結果の表示 ---
        // win.draw_text("CAP REG (Raw):", 20, 230, 14.0, 0xAAAAAA, f);
        // win.draw_hex(cap_reg as u64 as u32, 200, 230, f);
        //
        // win.draw_text("CAPLENGTH:", 20, 260, 14.0, 0xFFFF00, f);
        // win.draw_hex(cap_length as u32, 200, 260, f);
        //
        // win.draw_text("HCIVERSION:", 20, 290, 14.0, 0xFFFF00, f);
        // win.draw_hex(hci_version as u32, 200, 290, f);

        // 次のステップのための情報を表示
        let op_reg_virt = xhci_virt + cap_length as u64;
        // win.draw_text("OP REG Start:", 20, 330, 14.0, 0x00FFFF, f);
        // win.draw_hex(op_reg_virt as u64 as u32, 200, 330, f);


        let usb_cmd_addr = op_reg_virt as *mut u32;
        let usb_sts_addr = (op_reg_virt + 0x04) as *mut u32;

        unsafe {
            // 現在のステータスを確認し、動いていたら止める
            let mut usb_cmd = core::ptr::read_volatile(usb_cmd_addr);
            usb_cmd &= !0x0001; // RS (Run/Stop) = 0
            core::ptr::write_volatile(usb_cmd_addr, usb_cmd);

            // 停止するまで待機 (USBSTS.HCH)
            while (core::ptr::read_volatile(usb_sts_addr) & 0x0001) == 0 {
                core::hint::spin_loop();
            }

            // リセット (HCRST = 1)
            core::ptr::write_volatile(usb_cmd_addr, 0x0002);

            // リセット完了を待機 (HCRST が 0 に戻るまで)
            while (core::ptr::read_volatile(usb_cmd_addr) & 0x0002) != 0 {
                core::hint::spin_loop();
            }

            // コントローラが準備完了(CNR=0)になるのを待つ
            while (core::ptr::read_volatile(usb_sts_addr) & 0x0800) != 0 {
                core::hint::spin_loop();
            }
        }
        // win.draw_text("Host Controller Reset Done!", 20, 370, 14.0, 0x00FF00, f);

        // --- 5. 最大スロット数・ポート数の確認 ---
        let hcs_params1 = unsafe { core::ptr::read_volatile((xhci_virt + 0x04) as *const u32) };
        let max_slots = (hcs_params1 & 0xFF) as u8;
        let max_ports = ((hcs_params1 >> 24) & 0xFF) as u8;

        // win.draw_text("Slots:", 20, 400, 14.0, 0xAAAAAA, f);
        // win.draw_hex(max_slots as u32, 100, 400, f);
        // win.draw_text("Ports:", 250, 400, 14.0, 0xAAAAAA, f);
        // win.draw_hex(max_ports as u32, 330, 400, f);

        // --- 6. 各種メモリ構造体の割り当てと設定 ---
        // 注意: 以下のメモリは 64バイト境界でアライメントされている必要があります。

        // --- 6. (A) DCBAA の設定修正 ---

        // 1. DCBAA 自体の確保（これはすでにやっていますね）
        let dcbaa_phys = memory::allocate_phys_64();
        let dcbaap_reg = (op_reg_virt + 0x30) as *mut u64;
        unsafe { core::ptr::write_volatile(dcbaap_reg, dcbaa_phys); }

        // 2. DCBAA の中身を真っ白にする（自分＝CPUが触るので HHDM を足す）
        let dcbaa_virt = (dcbaa_phys + hhdm_offset) as *mut u64;
        unsafe {
            core::ptr::write_bytes(dcbaa_virt as *mut u8, 0, 2048);
        }

        // 3. 【ここが運命の分かれ道】Slot 1 用の Device Context を確保
        let device_context_phys = memory::allocate_phys_64(); // ← 物理アドレスを取得
        let device_context_virt = (device_context_phys + hhdm_offset) as *mut u8;

        // 4. Device Context を真っ白にする
        unsafe {
            core::ptr::write_bytes(device_context_virt, 0, 4096);
        }

        // 5. 【真犯人はここだ！】DCBAA[1] に「物理アドレス」を書き込む
        unsafe {
            // 物理アドレスをそのまま書く！ HHDM を足してはいけない！
            core::ptr::write_volatile(dcbaa_virt.add(1), device_context_phys);
        }

        // --- この後に RUN させて Address Device を投げる ---

        // (B) Command Ring
        // コマンド（TRB）を並べるリングバッファ
        let cmd_ring_phys = memory::allocate_phys_64();
        let crcr_reg = (op_reg_virt + 0x18) as *mut u64;
        // 下位1ビットを 1 (Ring Cycle State) にして書き込む
        unsafe { core::ptr::write_volatile(crcr_reg, cmd_ring_phys | 1); }

        // (C) 最大スロット数の設定 (CONFIGレジスタ)
        let config_reg = (op_reg_virt + 0x38) as *mut u32;
        unsafe {
            let mut conf = core::ptr::read_volatile(config_reg);
            conf = (conf & !0xFF) | (max_slots as u32);
            core::ptr::write_volatile(config_reg, conf);
        }

        // win.draw_text("DCBAA & Command Ring Set.", 20, 440, 14.0, 0x00FF00, f);

        // --- 7. ついに始動 (RUN) ---
        unsafe {
            let mut usb_cmd = core::ptr::read_volatile(usb_cmd_addr);
            usb_cmd |= 0x0001; // RS = 1
            core::ptr::write_volatile(usb_cmd_addr, usb_cmd);
        }
        // win.draw_text("xHCI is now RUNNING!", 20, 480, 14.0, 0xFFFF00, f);

        // --- 8. Runtime Register の特定 ---
        let rtsoff = unsafe { core::ptr::read_volatile((xhci_virt + 0x18) as *const u32) };
        let runtime_base = xhci_virt + (rtsoff & !0x1F) as u64;

        for i in 0..16 { // とりあえず64KB分 (16ページ)
            let offset = i * 4096;
            unsafe {
                page_manager.map_page(xhci_virt + offset, xhci_phys_base + offset, 0x1B);
            }
        }



        // --- 9. メモリ確保とゼロ初期化 ---
        let event_ring_phys = memory::allocate_phys_64();
        let erst_phys = memory::allocate_phys_64();


        if (event_ring_phys & 0xFFF) != 0 || (erst_phys & 0xFFF) != 0 {
            // もしここを通るならアロケータのアライメント設定が怪しい
            win.draw_text("Alignment Error!", 20, 380, 14.0, 0xFF0000, f);
            manager.add_window(win);
            return;
        }

        unsafe {
            // 確保したメモリを HHDM 経由でゼロクリア（ゴミデータによる暴走防止）
            core::ptr::write_bytes((event_ring_phys + hhdm_offset) as *mut u8, 0, 4096);
            core::ptr::write_bytes((erst_phys + hhdm_offset) as *mut u8, 0, 4096);



            let erst_virt = (erst_phys + hhdm_offset) as *mut u64;
            // ERST [0]: 物理アドレス, [1]: サイズ(256)
            core::ptr::write_volatile(erst_virt, event_ring_phys);
            core::ptr::write_volatile(erst_virt.add(1), 256); // 4096/16




            // // --- 10. Interrupter 0 設定 (書き込み順序を厳守) ---
             let ir0_base = runtime_base + 0x20;

            //落ちる..いや、かつて"落ちてた"ところ!

            core::ptr::write_volatile(ir0_base as *mut u32, 0);

            // 2. ERST Size を設定 (1セグメント)
            // 予約ビットを壊さないように、念のため下位16bit以外は保持
            let mut erstsz = core::ptr::read_volatile((ir0_base + 0x08) as *mut u32);
            erstsz = (erstsz & !0xFFFF) | 1;
            core::ptr::write_volatile((ir0_base + 0x08) as *mut u32, erstsz);

            // 3. ERST Dequeue Pointer (64bit)
            // 32bit環境や厳格なエミュレータに配慮して、下位・上位を分けて書く
            let erdp_ptr = (ir0_base + 0x18) as *mut u32;
            core::ptr::write_volatile(erdp_ptr, event_ring_phys as u32);
            core::ptr::write_volatile(erdp_ptr.add(1), (event_ring_phys >> 32) as u32);

            // 4. ERST Base Address (64bit)
            // これを書いた瞬間に xHCI がテーブルを読みに行くので、最後に書くのが定石
            let erstba_ptr = (ir0_base + 0x10) as *mut u32;
            core::ptr::write_volatile(erstba_ptr, erst_phys as u32);
            core::ptr::write_volatile(erstba_ptr.add(1), (erst_phys >> 32) as u32);

            // 5. 最後に IMAN で IE (Interrupt Enable) を立てる
            // ※ Bit 0 (IP) は RW1C なので、1を書いてクリアしておく（初期化の儀式）
            core::ptr::write_volatile(ir0_base as *mut u32, 0x0000_0003);


            // win.draw_text("Interrupter 0 Configured!", 20, 380, 14.0, 0x00FF00, f);



            // B. Dequeue Pointer を設定 (下位3ビットは 0 でOK)
            // 実機では 0x08 を付けない方が安全
            core::ptr::write_volatile((ir0_base + 0x18) as *mut u64, event_ring_phys);

            // C. 最後に Base Address を設定 (これでコントローラがテーブルを認識する)
            core::ptr::write_volatile((ir0_base + 0x10) as *mut u64, erst_phys);

            // D. 割り込みを有効化 (まずは IE ビットだけ立てる)
            let iman_addr = ir0_base as *mut u32;
            core::ptr::write_volatile(iman_addr, 0x0000_0002); // IE (Enable) のみ
        }

        // win.draw_text("Event Ring Initialized (v2)!", 20, 500, 14.0, 0x00FF00, f);


        // --- 11. Enable Slot コマンドの作成 ---
        // Command Ring の先頭（仮想アドレス）を取得
        let cmd_ring_virt = (cmd_ring_phys + hhdm_offset) as *mut u32;

        unsafe {
            // Enable Slot Command TRB の構造 (Spec 6.4.3.9)
            // [0-31]  : Reserved (0)
            // [32-63] : Reserved (0)
            // [64-95] : Reserved (0)
            // [96-127]: [TRB Type: 9 (Enable Slot)] | [Control Bits]

            core::ptr::write_volatile(cmd_ring_virt.add(0), 0);
            core::ptr::write_volatile(cmd_ring_virt.add(1), 0);
            core::ptr::write_volatile(cmd_ring_virt.add(2), 0);

            // TRB Type 9 は "Enable Slot"
            // Cycle Bit (Bit 0) を 1 にして「有効なデータだよ」と伝える
            let trb_type = 9;
            core::ptr::write_volatile(cmd_ring_virt.add(3), (trb_type << 10) | 1);
        }

        // Doorbell Register の場所を特定 (DBOFF は Capability Reg 0x14 にある)
        let dboff = unsafe { core::ptr::read_volatile((xhci_virt + 0x14) as *const u32) };
        let db_base = xhci_virt + dboff as u64;

        // Host Controller への Doorbell は 0 番
        let db0_ptr = db_base as *mut u32;

        // 0 を書き込むと「Command Ring に新しい TRB が入ったぞ」という合図になる
        unsafe {
            core::ptr::write_volatile(db0_ptr, 0);
        }

        win.draw_text("Waiting for Command Completion...", 20, 440, 14.0, 0xAAAAAA, f);

        // Event Ring の先頭を監視
        let event_ring_virt = (event_ring_phys + hhdm_offset) as *mut u32;

        let mut slot_id;

        loop {
            // Event TRB の 4つ目の u32 (index 3) の Bit 0 (Cycle Bit) が 1 になるのを待つ
            let status = unsafe { core::ptr::read_volatile(event_ring_virt.add(3)) };

            if (status & 0x01) != 0 {
                // 返ってきた！
                let completion_code = (unsafe { core::ptr::read_volatile(event_ring_virt.add(2)) } >> 24) & 0xFF;
                slot_id = (status >> 24) & 0xFF; // 割り振られた Slot ID

                if completion_code == 1 { // Success!
                    win.draw_text("Success! Slot ID:", 20, 470, 14.0, 0x00FF00, f);
                    win.draw_hex(slot_id, 180, 470, f);
                } else {
                    win.draw_text("Command Failed. Code:", 20, 470, 14.0, 0xFF0000, f);
                    win.draw_hex(completion_code, 200, 470, f);
                }
                break;
            }

            // QEMUなら一瞬ですが、実機だとわずかに時間がかかる場合があるので
            // 本来はタイムアウト処理が必要ですが、デバッグ中は無限ループでOK
        }

        // --- 12. Input Context の確保 ---
        // Input Context は最低でも 33 * 32 バイト (約1KB) 必要です
        // これも 64バイト境界 or 4KB境界である必要があります
        let input_context_phys = memory::allocate_phys_64(); // 4KB確保
        let input_context_virt = (input_context_phys + hhdm_offset) as *mut u32;

        unsafe {
            // 全て 0 で初期化
            core::ptr::write_bytes(input_context_virt as *mut u8, 0, 4096);

            // --- A. Input Control Context (先頭 32 bytes) ---
            // どの設定を有効にするか指定。今回は「Slot」と「Endpoint 0」を有効にする
            // Bit 0 = Drop Context (今回は使わないので 0)
            // Bit 1 = Add Context (Slot)
            // Bit 2 = Add Context (Endpoint 0)
            core::ptr::write_volatile(input_context_virt.add(1), 0x0000_0006); //Gemini
            //core::ptr::write_volatile(input_context_virt.add(1), 0x3); //ChatGPT
            // --- B. Slot Context (次 32 bytes) ---
            // デバイス全体の情報を書く
            let slot_ctx = input_context_virt.add(16); // インデックス8から //64bitなら16がただしい!!!
            // Route String=0, Speed=(さっき調べた値), Context Entries=1 (EP0のみ)
            // ここではとりあえず汎用的な値をセット
            let context_entries = 1;
            let speed = 3; // とりあえず SuperSpeed (本来は PORTSC から取得)
            core::ptr::write_volatile(slot_ctx.add(0), (context_entries << 27) | (speed << 20));
            // Root Hub Port Number (刺さっているポート番号)
            core::ptr::write_volatile(slot_ctx.add(5), (5 << 16)); // ポート1と仮定 //5だよ5！！！

            // --- C. Endpoint 0 Context (次 32 bytes) ---
            // USBの制御用エンドポイントの設定
            let ep0_ctx = input_context_virt.add(32); // インデックス16から　//64bitなら32!
            // EP Type = Control, Max Packet Size = 512 (SuperSpeed時)
            core::ptr::write_volatile(ep0_ctx.add(1), (4 << 3) | (64 << 16));
            // ここに Transfer Ring (データ転送用リング) のアドレスを書く必要がある...
        }

        // win.draw_text("Input Context Prepared!", 20, 40, 14.0, 0x00FF00, f);

        // --- 13. Transfer Ring (EP0用) の確保 ---
        let ep0_ring_phys = memory::allocate_phys_64(); // 4KB確保
        let ep0_ring_virt = (ep0_ring_phys + hhdm_offset) as *mut u32;

        unsafe {
            // ゼロクリア
            core::ptr::write_bytes(ep0_ring_virt as *mut u8, 0, 4096);
        }

        // --- 14. Input Context に Transfer Ring の住所を書く ---
        // さっき作った ep0_ctx (index 16番) に、このリングのアドレスを紐付ける
        unsafe {
            let ep0_ctx = input_context_virt.add(32);

            // TR Dequeue Pointer (64bit)
            // Bit 0 は DCS (Dequeue Cycle State)。最初は 1 にしておくのが定石
            core::ptr::write_volatile(ep0_ctx.add(2), ep0_ring_phys as u32 | 1);
            core::ptr::write_volatile(ep0_ctx.add(3), (ep0_ring_phys >> 32) as u32);

            // 平均 TRB 長 (通常 8 でOK)
            core::ptr::write_volatile(ep0_ctx.add(4), 8);
        }

        // win.draw_text("EP0 Transfer Ring Linked!", 20, 70, 14.0, 0x00FF00, f);

        // --- 15. Address Device コマンドの投下 ---
        // Command Ring の次の空きスロット（今は index 1 と仮定）を使用
        let mut ad_trb_virt;

        unsafe {
            //ad_trb_virt = cmd_ring_virt.add(4); // 1 TRB = 4 * u32 なので次は index 4
            ad_trb_virt = cmd_ring_virt;

            // Input Context の物理アドレスをセット
            // core::ptr::write_volatile(ad_trb_virt.add(0), input_context_phys as u32);
            // core::ptr::write_volatile(ad_trb_virt.add(1), (input_context_phys >> 32) as u32);
            //
            // // Status (Reserved)
            // core::ptr::write_volatile(ad_trb_virt.add(2), 0);

            core::ptr::write_volatile(ad_trb_virt.add(0), input_context_phys as u32);
            core::ptr::write_volatile(ad_trb_virt.add(1), (input_context_phys >> 32) as u32);
            core::ptr::write_volatile(ad_trb_virt.add(2), 0);

            // Control bits
            let trb_type = 11; // Address Device
            let bsr = 0;       // ガチのモードｗ
            let cycle = 1;
            let slot_id_u32 = slot_id as u32;

            let crcr_reg = (op_reg_virt + 0x18) as *mut u64;

            core::ptr::write_volatile(
                ad_trb_virt.add(3),
                (slot_id_u32 << 24) | (trb_type << 10) | (bsr << 9) | cycle
            );

            core::ptr::write_volatile(crcr_reg, cmd_ring_phys | 0x01);

            core::arch::x86_64::_mm_mfence();

        }

        // ドアベルを鳴らしてコントローラに通知
        unsafe {
            core::ptr::write_volatile(db0_ptr, 0);
        }

        win.draw_text("Address Device (BSR=1) Sent. Waiting...", 20, 510, 14.0, 0xAAAAAA, f);

        // Event Ring の 2つ目の TRB (index 4〜7) を監視
        // ※Enable Slot のイベントが index 0〜3 に入っている前提

        let event_ptr;
        //unsafe { event_ptr = event_ring_virt.add(4);}
        unsafe { event_ptr = event_ring_virt;}

        loop {
            let status = unsafe { core::ptr::read_volatile(event_ptr.add(3)) };

            // Cycle Bit が 1 になるのを待つ
            if (status & 0x01) != 0 {
                let completion_code = (unsafe { core::ptr::read_volatile(event_ptr.add(2)) } >> 24) & 0xFF;
                let event_type = (status >> 10) & 0x3F;

                // Command Completion Event (Type 33) であることを確認
                if event_type == 33 {
                    if completion_code == 1 {
                        win.draw_text("ADDRESS DEVICE SUCCESS!", 20, 540, 16.0, 0x00FF00, f);
                        win.draw_text("The long-time enemy is defeated.", 20, 570, 12.0, 0x00FF00, f);
                    } else {
                        win.draw_text("ADDRESS DEVICE FAILED...", 20, 540, 16.0, 0xFF0000, f);
                        win.draw_text("Code:", 20, 570, 14.0, 0xFFFFFF, f);
                        win.draw_hex(completion_code, 80, 570, f);

                        // 失敗した時のヒント
                        if completion_code == 17 { // Context State Error
                            win.draw_text("Hint: Check Context State/Entries", 20, 600, 12.0, 0xAAAAAA, f);
                        } else if completion_code == 19 { // Parameter Error
                            win.draw_text("Hint: Check Alignment or BSR", 20, 600, 12.0, 0xAAAAAA, f);
                        }
                    }
                }
                break;
            }
            core::hint::spin_loop();
        }

        // --- デバッグダンプ開始 ---

        // 1. 根本的な設定の確認 (HCCPARAMS1)
        // let hccp1 = unsafe { core::ptr::read_volatile((xhci_virt + 0x08) as *const u32) };
        // let csz = hccp1 & 0x01; // Bit 0: Context Size (0=32byte, 1=64byte)
        //
        // win.draw_text("HCCPARAMS1:", 20, 100, 14.0, 0xAAAAAA, f);
        // win.draw_hex(hccp1, 150, 100, f);
        // win.draw_text(if csz == 1 { "CSZ: 64-byte" } else { "CSZ: 32-byte" }, 280, 100, 14.0, 0xFF00FF, f);
        //
        // // 2. Doorbell 住所の再確認
        // win.draw_text("DB Offset:", 20, 130, 14.0, 0xAAAAAA, f);
        // win.draw_hex(dboff, 150, 130, f);
        // win.draw_text("DB0 Ptr:", 280, 130, 14.0, 0xAAAAAA, f);
        // win.draw_hex(db0_ptr as u64 as u32, 380, 130, f);
        //
        // // 3. Command Ring の現在地 (CRCR)
        // let crcr = unsafe { core::ptr::read_volatile((op_reg_virt + 0x18) as *const u64) };
        // win.draw_text("CRCR Reg:", 20, 160, 14.0, 0xAAAAAA, f);
        // win.draw_hex((crcr >> 32) as u32, 150, 160, f); // 上位
        // win.draw_hex(crcr as u32, 250, 160, f);         // 下位

        // 4. Address Device TRB の生データ (投げた直後のメモリ)
        // cmd_ring_virt.add(4) 周辺をダンプ
        // win.draw_text("AD TRB Raw:", 20, 200, 14.0, 0xFFFF00, f);
        // for i in 4..8 {
        //     let val = unsafe { core::ptr::read_volatile(cmd_ring_virt.add(i)) };
        //     win.draw_hex(val, (20 + (i - 4) * 100) as i32, 220, f);
        // }

        // 5. Event Ring の生ダンプ (ここが一番重要)
        // index 0..3 (Enable Slot), 4..7 (Address Device)
        // win.draw_text("Event Ring Dump (Index 0-7):", 20, 260, 14.0, 0x00FFFF, f);
        // for i in 0..8 {
        //     let val = unsafe { core::ptr::read_volatile(event_ring_virt.add(i)) };
        //     let y = 280 + (i / 4) * 30;
        //     let x = 20 + (i % 4) * 100;
        //     win.draw_hex(val, x as i32, y as i32, f);
        // }

        // 6. Input Context の先頭を少しだけ
        // win.draw_text("Input Context Head:", 20, 360, 14.0, 0xAAAAAA, f);
        // for i in 0..4 {
        //     let val = unsafe { core::ptr::read_volatile(input_context_virt.add(i)) };
        //     win.draw_hex(val, (20 + i * 100) as i32, 380, f);
        // }

        // --- 現場検証ダンプ ---
        // 1. TRBの4つのDwordを表示
        // win.draw_text("TRB DW3:", 20, 600, 12.0, 0xFFFFFF, f);
        // win.draw_hex(unsafe { core::ptr::read_volatile(ad_trb_virt.add(3)) }, 100, 600, f);
        //
        // // 2. Input Contextのアドレスを表示 (アライメント確認)
        // win.draw_text("InpCtx Phys:", 20, 620, 12.0, 0xFFFFFF, f);
        // win.draw_hex(input_context_phys as u32, 120, 620, f); // 下位32bit
        // win.draw_hex((input_context_phys >> 32) as u32, 250, 620, f); // 上位32bit
        //
        // // 3. Input Contextの中身 (重要ビット)
        // let ctrl_ctx_dw1 = unsafe { core::ptr::read_volatile(input_context_virt.add(1)) };
        // win.draw_text("Ctrl DW1:", 20, 640, 12.0, 0xFFFFFF, f);
        // win.draw_hex(ctrl_ctx_dw1, 100, 640, f); // 0x00000006 なら正解

        // --- デバッグダンプ終了 ---

        // --- 現場検証：ポートの正体を暴く ---

        // --- ポート5の正体を暴く！ ---

        // 1. Operational Register までは同じ
        let caplength = unsafe { core::ptr::read_volatile(xhci_virt as *const u8) } as u64;
        let op_base = xhci_virt + caplength;

        // 2. Port Register Set 5 の場所を計算
        // ポートnのアドレス = PortRegisterBase + (n - 1) * 0x10
        // なので、ポート5なら 0x400 + (4 * 0x10) = 0x440
        let port5_offset = 0x400 + (4 * 0x10);
        let portsc_ptr = (op_base + port5_offset) as *const u32;

        // 3. レジスタを読み取る
        let portsc = unsafe { core::ptr::read_volatile(portsc_ptr) };

        // CCS (Bit 0) もついでに確認して表示すると安心です
        let is_connected = (portsc & 0x01) != 0;

        // 4. スピード（何速）を抽出 (Bit 10-13)
        let speed_id = (portsc >> 10) & 0x0F;

        // win.draw_text("Port 5 Speed ID:", 20, 500, 14.0, 0xFFFFFF, f);
        // win.draw_hex(speed_id, 180, 500, f);

        // 意味を表示
        let speed_msg = if !is_connected {
            "Not Connected"
        } else {
            match speed_id {
                1 => "Full-Speed",
                2 => "Low-Speed",
                3 => "High-Speed",
                4 => "SuperSpeed",
                _ => "?? (Unknown Speed)",
            }
        };
        //win.draw_text(speed_msg, 240, 500, 14.0, 0xFFFF00, f);

        // こんな感じで一気に並べて出すと「ズレ」が視覚的にわかります
        for i in 0..48 {
            let val = unsafe { core::ptr::read_volatile(input_context_virt.add(i)) };
            let x = (i % 8) * 80 + 20;
            let y = (i / 8) * 20 + 100; // 適当な表示位置
            win.draw_hex(val, x as i32, y as i32, f);
        }



        manager.add_window(win);

    }


}