use crate::println;
use core::arch::asm;


#[repr(C, packed)]
pub struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access: u8,
    limit_high_flags: u8,
    base_high: u8,
}

impl GdtEntry {
    pub const fn new(access: u8, flags: u8) -> Self {
        GdtEntry {
            limit_low: 0xFFFF,        // 64bitモードでは無視されるが0xFFFFが通例
            base_low: 0,
            base_mid: 0,
            access,                   // 0x9A (Code), 0x92 (Data)
            limit_high_flags: flags,  // 0xAF (Code), 0xCF (Data)
            base_high: 0,
        }
    }
}

#[repr(C, packed)]
pub struct GdtPointer {
    limit: u16,
    base: u64,
}

// 64bitモードで最低限必要なGDT（Null, KernelCode, KernelData）
#[repr(C, align(16))] // 16バイト境界に強制配置
struct GdtTable([u64; 3]);

static GDT: GdtTable = GdtTable([
    0x0000000000000000,
    0x00af9a000000ffff,
    0x00cf92000000ffff,
]);

// pub fn init() {
//     // 1. GDTを「確実」なスタック領域に確保（ページフォールト回避）
//     let local_gdt: [u64; 3] = [
//         0x0000000000000000, // Null
//         0x00af9a000000ffff, // 0x08: Code
//         0x00cf92000000ffff, // 0x10: Data
//     ];
//     let limit = (core::mem::size_of_val(&local_gdt) - 1) as u16;
//     let base = local_gdt.as_ptr() as u64;
//
//     unsafe {
//         asm!(
//         // GDTR（ポインタ）をスタックに積んでロード
//         "sub rsp, 16",
//         "mov [rsp + 2], {base_reg}",
//         "mov [rsp], {limit_reg:x}",
//         "lgdt [rsp]",
//         "add rsp, 16",
//
//         // 運命のセグメント切り替え
//         "push {cs_val}",          // 0x08を8バイトでプッシュ
//         "lea {tmp}, [rip + 2f]",  // ジャンプ先（ラベル2:）のアドレス
//         "push {tmp}",             // アドレスを8バイトでプッシュ
//         "retfq",                  // ここで魂が入れ替わる
//         "2:",                     // 転生先
//
//
//         // データセグメントも一応合わせておく
//         "mov ax, 0x10",
//         "mov ds, ax",
//         "mov es, ax",
//         "mov ss, ax",
//
//         base_reg = in(reg) base,
//         limit_reg = in(reg) limit,
//         cs_val = in(reg) 0x08u64, // 型をu64にして8バイトを保証
//         tmp = out(reg) _,
//         );
//     }
// }

// gdt.rs
pub fn init(offset: i64) {
    unsafe {
        // 1. GDTの仮想アドレスを LEA で取得
        let gdt_ptr_virt: u64;
        asm!("lea {}, [rip + {}]", out(reg) gdt_ptr_virt, sym GDT);

        // 2. 物理アドレスへ変換 (virt + offset)
        let gdt_ptr_phys = (gdt_ptr_virt as i64 + offset) as u64;

        // 3. 次に飛ぶ場所（ラベル 2:）の仮想アドレスを物理へ変換
        let jump_target_virt: u64;
        asm!("lea {}, [rip + 2f]", out(reg) jump_target_virt);
        let jump_target_phys = (jump_target_virt as i64 + offset) as u64;

        let limit = (core::mem::size_of_val(&GDT) - 1) as u16;

        // 4. 物理アドレスを CPU に突きつける
        asm!(
        "sub rsp, 16",
        "mov [rsp + 2], {base}",
        "mov [rsp], {limit:x}",
        "lgdt [rsp]",
        "add rsp, 16",

        "push {cs_val}",
        "push {target}",
        "retfq",
        "2:",

        "mov ax, 0x10",
        "mov ds, ax",
        "mov es, ax",
        "mov ss, ax",
        "mov fs, ax",
        "mov gs, ax",

        base = in(reg) gdt_ptr_phys,
        limit = in(reg) limit,
        cs_val = in(reg) 0x08u64,
        target = in(reg) jump_target_phys,
        );
    }
    println("GDT: Initialized!");
}