use core::arch::asm;
use core::arch::x86_64::_rdtsc;

// 静的変数：メモリ管理（ヒープ）がなくても静的領域に配置されます
static mut BASE_TSC: u64 = 0;   // カーネル起動時のTSC
static mut TSC_FREQ: u64 = 0;   // RTCで実測した1秒あたりのTSC

/// I/Oポート操作（inb/outb）
unsafe fn _outp(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}

unsafe fn _inp(port: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    value
}

/// RTCの「秒」を読み取る（更新中フラグ UIP を考慮して正確に待機）
fn read_rtc_seconds() -> u8 {
    unsafe {
        // ステータスレジスタAのUIPフラグが0（更新中でない）になるまで待つ
        loop {
            _outp(0x70, 0x0A);
            if (_inp(0x71) & 0x80) == 0 { break; }
        }
        _outp(0x70, 0x00); // インデックス0: 秒
        _inp(0x71)
    }
}

/// 🚀 カーネルの最序盤で一度だけ呼び出す（起点を記録）
pub fn record_start_time() {
    unsafe {
        BASE_TSC = _rdtsc();
    }
}

/// 周波数を実測し、タイマーを有効化する
pub fn init() {
    // 1. RTCの秒が切り替わる瞬間を待つ（計測開始の同期）
    let s1 = read_rtc_seconds();
    while read_rtc_seconds() == s1 {
        core::hint::spin_loop();
    }
    let t1 = unsafe { _rdtsc() };

    // 2. 次の秒が切り替わる瞬間を待つ（正確に1秒間）
    let s2 = read_rtc_seconds();
    while read_rtc_seconds() == s2 {
        core::hint::spin_loop();
    }
    let t2 = unsafe { _rdtsc() };

    unsafe {
        TSC_FREQ = t2 - t1;
    }
}

/// カーネル起動（record_start_time実行）からの経過ミリ秒を返す
pub fn get_uptime_ms() -> u64 {
    unsafe {
        if TSC_FREQ == 0 {
            // init前は暫定的に0を返す（または予測値 2_400_000_000 で計算）
            return 0;
        }
        let elapsed = _rdtsc() - BASE_TSC;
        // 精度維持のため先に1000を掛けてから割る
        (elapsed * 1000) / TSC_FREQ
    }
}

/// 指定したミリ秒だけビジーウェイトする
pub fn sleep_ms(ms: u64) {
    let target = get_uptime_ms() + ms;
    while get_uptime_ms() < target {
        core::hint::spin_loop();
    }
}