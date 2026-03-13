use crate::{print_hex, println, TICK_COUNT};
use core::arch::asm;
use core::sync::atomic::Ordering;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct IdtEntry {
    offset_low: u16,      // ハンドラアドレス 0-15
    selector: u16,        // GDTのコードセグメント (0x08)
    ist: u8,              // Interrupt Stack Table (とりあえず0)
    type_attr: u8,        // 0x8E (Interrupt Gate, Ring 0)
    offset_mid: u16,      // ハンドラアドレス 16-31
    offset_high: u32,     // ハンドラアドレス 32-63
    reserved: u32,        // 予約 (0)
}

impl IdtEntry {
    pub const fn new() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    pub fn set_handler(&mut self, handler_addr: u64) {
        self.selector = 0x08; // GDTのKernel Codeを選択
        self.type_attr = 0x8E; // 存在する + Ring 0 + Interrupt Gate
        self.offset_low = handler_addr as u16;
        self.offset_mid = (handler_addr >> 16) as u16;
        self.offset_high = (handler_addr >> 32) as u32;
    }
}

#[repr(C, packed)]
struct Idtr {
    limit: u16,
    base: u64,
}

#[repr(C, packed)]
pub struct InterruptStackFrame {
    pub instruction_pointer: u64, // どこで死んだか（RIP）
    pub code_segment: u64,
    pub cpu_flags: u64,
    pub stack_pointer: u64,
    pub stack_segment: u64,
}

// IDT本体（256個のエントリ）
static mut IDT: [IdtEntry; 256] = [IdtEntry::new(); 256];

// idt.rs
pub fn init(offset: i64) {
    unsafe {
        // 1. 仮想アドレスを物理アドレスに変換するヘルパー
        // offset = phys_base - virt_base なので、足すだけで物理アドレスになります
        let to_phys = |virt: u64| -> u64 {
            (virt as i64 + offset) as u64
        };

        IDT[0].set_handler(divide_by_zero_handler as u64);
        IDT[14].set_handler(page_fault_handler as u64);
        IDT[32].set_handler(timer_handler as u64);

        // 3. IDT自体の仮想アドレスを LEA で取得
        let idt_virt: u64;
        asm!("lea {}, [rip + {}]", out(reg) idt_virt, sym IDT);

        // 4. IDTRを構築
        // ※注意: size_of_val(&IDT) に修正（&&raw const IDT だとポインタのサイズになってしまいます）
        let idtr = Idtr {
            // limit: (core::mem::size_of_val(&IDT) - 1) as u16, // これを以下に変更
            limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: to_phys(idt_virt),
        };

        // 5. CPUにロード
        asm!("lidt [{}]", in(reg) &idtr);
    }

    println("IDT: Initialized!");

}

// --- ハンドラの実装 ---
// 割り込みハンドラは特殊な呼び出し規約が必要
// ※ main.rs の一番上に #![feature(abi_x86_interrupt)] が必要です

extern "x86-interrupt" fn divide_by_zero_handler(frame: InterruptStackFrame) {

    println("!!! [EXCEPTION] DIVIDE BY ZERO !!!");

    // せっかくなので、どこで死んだか表示してみる
    print_hex(frame.instruction_pointer);


    loop{ unsafe { asm!("hlt") } }
}

// ページフォルトなどの「エラーコード」が出るタイプは引数が2つ必要
extern "x86-interrupt" fn page_fault_handler(frame: InterruptStackFrame, error_code: u64) {
    println("!!! [EXCEPTION] PAGE FAULT !!!");

    print_hex(frame.instruction_pointer);
    print_hex(error_code);
    loop {}
}

// ハンドラ
extern "x86-interrupt" fn timer_handler(_frame: InterruptStackFrame) {
    // 1. カウントアップ
    TICK_COUNT.fetch_add(1, Ordering::Relaxed);

    // 2. EOI (End of Interrupt) を送る
    // これを送らないと、CPUは「まだ前の割り込みが終わってない」と思って
    // 次のタイマー割り込みを受け付けてくれません。
    unsafe {
        // LAPICのレジスタ（通常 0xfee000b0）に0を書き込む
        // ※ すでにAPICの構造体やアドレスを定義している場合はそれを使ってください
        core::ptr::write_volatile(0xfee000b0 as *mut u32, 0);
    }
}