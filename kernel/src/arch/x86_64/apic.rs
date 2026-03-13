pub fn init_apic_timer(hhdm_offset: u64) { unsafe {
    let lapic_base = (0xfee00000 + hhdm_offset) as *mut u32;

    // ヘルパー関数: 指定オフセットのレジスタに書き込む
    let write_reg = |offset: usize, value: u32| {
        lapic_base.add(offset / 4).write_volatile(value);
    };

    // 1. 分周比を16に設定
    write_reg(0x3e0, 0x03);

    // 2. タイマーの設定 (Periodicモード + 割り込み番号32)
    write_reg(0x320, 0x20 | 0x20000);


    // 3. 初期カウント値を設定してスタート！
    write_reg(0x380, 0x10000); //10000くらいがちょうどいい
}}