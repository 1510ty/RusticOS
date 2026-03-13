#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

extern crate alloc;
mod font;
mod drawstr;
mod vga;
mod draw;
mod memory;

#[cfg(target_arch = "x86_64")]
mod arch {
    pub mod x86_64;
}

use alloc::string::String;
use crate::arch::x86_64::apic::init_apic_timer;
use crate::draw::{print_hex, print_usable_memory_stats, println};
use crate::memory::init_heap;
use crate::vga::{clear_back_buffer, init_vga, is_empty, update_screen};
use alloc::vec::Vec;
use core::arch::asm;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU64, Ordering};
use limine::request::{ExecutableAddressRequest, FramebufferRequest, HhdmRequest, MemoryMapRequest};
use linked_list_allocator::LockedHeap;

//Limineからの情報取得
#[used]
pub static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();
pub static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();
pub static EXECUTABLE_ADDRESS_REQUEST: ExecutableAddressRequest = ExecutableAddressRequest::new();
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();


pub static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

//ダブルバッファリング系
pub static mut FRAMEBUFFER_BACK: Option<Vec<u32>> = None;
pub static mut SCREEN_WIDTH: usize = 0;
pub static mut SCREEN_HEIGHT: usize = 0;

//static mut GLOBAL_BACK_BUFFER: Option<Vec<u32>> = None;

pub static mut CURRENT_Y: u64 = 0;

pub static mut NEEDS_FRAME_UPDATE: bool = false;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();


#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {

    //割り込み禁止
    #[cfg(target_arch = "x86_64")]
    unsafe{asm!("cli")}

    let hhdm_offset = HHDM_REQUEST.get_response().unwrap().offset();
    init_heap(hhdm_offset);

    init_vga();

    clear_back_buffer(0xFFFFFF);

    println("Starting RusticOS...");
    println("RusticOSを起動しています...");

    println("Limineから情報を取得しています...");
    //Limineへの実行アドレスのリクエスト
    let response = EXECUTABLE_ADDRESS_REQUEST.get_response().unwrap();

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

    init_apic_timer(hhdm_offset);

    println("Timer Start!");

    //割り込み許可
    #[cfg(target_arch = "x86_64")]
    unsafe{asm!("sti")}

    println("WELCOME TO RUSTIC OS!");
    println("Rustic OS へようこそ!");

    //print_usable_memory_stats(mmap);

    loop {
        // 1. 注文（キュー）があれば即座に調理（レンダリング）
        // TICK_COUNTを待たずに、何か届いたらすぐ描画するのが今の主流！
        if !is_empty() {
            update_screen();
        }

        // 2. テスト用の「自爆スイッチ」を残すならここ
        /*
        let t = TICK_COUNT.load(Ordering::Relaxed);
        if t > 5000 { // 5秒後くらいに爆発
             unsafe { asm!("xor rax, rax; div rax"); }
        }
        */

        // 3. 何もすることがなければ眠る
        // 割り込み（タイマーやキーボード）が入るまでCPUを休ませる
        unsafe { asm!("hlt") };
    }
}
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { unsafe { asm!("hlt") } }
}