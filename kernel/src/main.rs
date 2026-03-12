#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]


mod font;
mod drawstr;


#[cfg(target_arch = "x86_64")]
mod arch {
    pub mod x86_64;
}

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use crate::drawstr::draw_str;
use core::arch::asm;
use core::panic::PanicInfo;
use core::ptr::copy_nonoverlapping;
use core::sync::atomic::{AtomicU64, Ordering};
use limine::framebuffer::Framebuffer;
use limine::memory_map::{Entry, EntryType};
use limine::request::{ExecutableAddressRequest, FramebufferRequest, HhdmRequest, MemoryMapRequest};
use linked_list_allocator::LockedHeap;
use spin::Mutex;
use crate::arch::x86_64::apic::init_apic_timer;

//Limineからの情報取得
#[used]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();
static EXECUTABLE_ADDRESS_REQUEST: ExecutableAddressRequest = ExecutableAddressRequest::new();
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();


pub static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

//ダブルバッファリング系
pub static mut FRAMEBUFFER_BACK: Option<Vec<u32>> = None;
pub static mut SCREEN_WIDTH: usize = 0;
pub static mut SCREEN_HEIGHT: usize = 0;
static mut GLOBAL_BACK_BUFFER: Option<Vec<u32>> = None;

static mut CURRENT_Y: u64 = 0;

pub static mut VRAM_PTR: *mut u32 = core::ptr::null_mut();

static mut NEEDS_FRAME_UPDATE: bool = false;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();


#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {

    //割り込み禁止
    #[cfg(target_arch = "x86_64")]
    unsafe{asm!("cli")}

    let hhdm_offset = HHDM_REQUEST.get_response().unwrap().offset();
    init_heap(hhdm_offset);

    //フレームバッファー(fb)取得とその取得したやつでpitchとか取得
    let fb_response = FRAMEBUFFER_REQUEST.get_response().unwrap();
    let fb = fb_response.framebuffers().next().expect("No framebuffer found");

    unsafe {
        VRAM_PTR = fb.addr() as *mut u32; // ここで「本物の住所」をメモ！
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    init_double_buffer(width, height);

    clear_back_buffer(0xFFFFFF);

    println("Starting RusticOS...");
    println("RusticOSを起動しています...");

    println("Limineから情報を取得しています...");
    //Limineへの実行アドレスのリクエスト
    let response = EXECUTABLE_ADDRESS_REQUEST.get_response()
        .expect("Limine request failed");


    //メモリマップ取得
    let mmap_response = MEMORY_MAP_REQUEST.get_response().unwrap();
    let mmap = mmap_response.entries();

    println("Limineから情報を取得しました!");


    #[cfg(target_arch = "x86_64")]
    arch::x86_64::init(response); // x86_64の時だけ実行
    #[cfg(target_arch = "x86_64")]
    println("GDT AND IDT OK!");


    let mmap_response = MEMORY_MAP_REQUEST.get_response().unwrap();
    let mmap = mmap_response.entries();




    println("メモリ確保の初期化が完了しました!");

    unsafe {
        init_apic_timer(hhdm_offset);
    }

    println("Timer Start!");

    //割り込み許可
    #[cfg(target_arch = "x86_64")]
    unsafe{asm!("sti")}

    println("WELCOME TO RUSTIC OS!");
    println("Rustic OS へようこそ!");

    // println("Triggering Divide by Zero...");
    // unsafe {
    //     asm!(
    //     "mov rax, 0",
    //     "div rax", // 0で割る！
    //     );
    // }

    print_usable_memory_stats(mmap);

    for _ in 0..10000 {
        clear_back_buffer(0xFF0000);
        clear_back_buffer(0x00FF00);
        clear_back_buffer(0x0000FF);
    }

    clear_back_buffer(0xFFFFFF);

    loop {
        // #[cfg(target_arch = "x86_64")]
        // let t = TICK_COUNT.load(Ordering::Relaxed);
        // if t % 100 == 0 { // 100回に1回表示
        //     //print_hex(t);
        // }
        // unsafe { asm!("hlt") };


    }
}

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

fn print_usable_memory_stats(mmap: &[&Entry]) {

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

pub fn init_heap(hhdm_offset: u64) {
    let mmap_response = MEMORY_MAP_REQUEST.get_response().unwrap();
    let mmap = mmap_response.entries();

    // 2. 「一番大きい空き領域」をヒープとして使う
    // 実機は断片化しているので、最大の領域を探すのが安全
    let usable_entry = mmap.iter()
        .filter(|e| e.entry_type == EntryType::USABLE)
        .max_by_key(|e| e.length)
        .expect("No usable memory found for heap");

    // 3. 物理アドレスを HHDM 上の仮想アドレスに変換
    let heap_start = usable_entry.base + hhdm_offset;
    let heap_size = usable_entry.length;

    unsafe {
        // 4. アロケータを初期化（ここで Rust がメモリを使えるようになる）
        ALLOCATOR.lock().init(heap_start as *mut u8, heap_size as usize);
    }
}

pub fn init_double_buffer(width: usize, height: usize) {
    unsafe {
        SCREEN_WIDTH = width;
        SCREEN_HEIGHT = height;

        // ヒープから (幅 * 高さ * 4バイト) の領域を確保
        // これで Vec が裏画面の実体として固定される
        FRAMEBUFFER_BACK = Some(vec![0u32; width * height]);

        // 念のため、最初は真っ黒（または好きな色）で塗りつぶしておく
        if let Some(ref mut back) = FRAMEBUFFER_BACK {
            back.fill(0x000000);
        }
    }
}

pub fn swap_buffers(fb_ptr: *mut u32) {
    unsafe {
        if let Some(ref back) = FRAMEBUFFER_BACK {
            // copy_nonoverlapping は Rust版 memcpy
            // 第3引数は「バイト数」ではなく「要素数(u32の数)」なのでこれでOK
            copy_nonoverlapping(
                back.as_ptr(),
                fb_ptr,
                SCREEN_WIDTH * SCREEN_HEIGHT
            );
        }
    }
}

// 画面を特定の色で塗りつぶす（リセット用）
pub fn clear_back_buffer(color: u32) {
    unsafe {
        if let Some(ref mut back) = FRAMEBUFFER_BACK {
            // slice::fill はループを回すより圧倒的に速い
            back.fill(color);
            request_update();
        }
    }
}

pub fn request_update() {
    unsafe {swap_buffers(VRAM_PTR);}
    //unsafe { NEEDS_FRAME_UPDATE = true; }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { unsafe { asm!("hlt") } }
}