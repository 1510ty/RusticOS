use core::arch::asm;

/// PCI構成空間から32ビット読み取る
pub fn pci_config_read_32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    // アドレス指定用の32ビット値を作成
    // Bit 31: Enable, 23-16: Bus, 15-11: Slot, 10-8: Func, 7-2: Offset
    let address = ((1u32 << 31) |
        ((bus as u32) << 16) |
        ((slot as u32) << 11) |
        ((func as u32) << 8) |
        ((offset as u32) & 0xFC));

    let mut data: u32;
    unsafe {
        // 0xCF8 (CONFIG_ADDRESS) にアドレスを書き込む
        asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") address);
        // 0xCFC (CONFIG_DATA) からデータを読み出す
        asm!("in eax, dx", out("eax") data, in("dx") 0xCFCu16);
    }
    data
}

/// PCI構成空間に32ビット書き込む
pub fn pci_config_write_32(bus: u8, slot: u8, func: u8, offset: u8, data: u32) {
    let address = ((1u32 << 31) |
        ((bus as u32) << 16) |
        ((slot as u32) << 11) |
        ((func as u32) << 8) |
        ((offset as u32) & 0xFC));
    unsafe {
        asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") address);
        asm!("out dx, eax", in("dx") 0xCFCu16, in("eax") data);
    }
}