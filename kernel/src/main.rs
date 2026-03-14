#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

extern crate alloc;
pub static FONT_DATA: &[u8] = include_bytes!("NotoSansJP-Regular.ttf");

mod font;
mod drawstr;
mod vga;
mod draw;
mod memory;
mod dwm;


#[cfg(target_arch = "x86_64")]
mod arch {
    pub mod x86_64;
}

use crate::arch::x86_64::apic::init_apic_timer;
use crate::arch::x86_64::timer;
use crate::draw::{print_hex, println};
use crate::memory::init_heap;
use crate::vga::{clear_back_buffer, init_vga, update_screen};
use alloc::string::ToString;
use alloc::vec::Vec;
use core::arch::asm;
use core::arch::x86_64::__cpuid;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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

static INITIALIZED: AtomicBool = AtomicBool::new(false);

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();


pub static mut VRAM_PTR: *mut u32 = core::ptr::null_mut();

pub static mut WIDTH: usize = 0;
pub static mut HEIGHT: usize = 0;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {

    //割り込み禁止
    #[cfg(target_arch = "x86_64")]
    unsafe{asm!("cli")}

    timer::record_start_time();

    let hhdm_offset = HHDM_REQUEST.get_response().unwrap().offset();
    init_heap(hhdm_offset);

    init_vga();

    clear_back_buffer(0xFFFFFF);

    println("Starting RusticOS...");

    println("Fetching information from Limine...");
    //Limineへの実行アドレスのリクエスト
    let response = EXECUTABLE_ADDRESS_REQUEST.get_response().unwrap();

    println("Done fetching Limine boot information.");

    #[cfg(target_arch = "x86_64")]
    arch::x86_64::init(response); // x86_64の時だけ実行
    #[cfg(target_arch = "x86_64")]
    println("GDT AND IDT OK!");

    println("Memory allocator initialized successfully.");

    println("Timer initializing...");
    timer::init();
    println("Timer initialized!");


    init_apic_timer(hhdm_offset);
    println("APCI Timer Start!");


    //割り込み許可
    #[cfg(target_arch = "x86_64")]
    unsafe{asm!("sti")}

    INITIALIZED.store(true, Ordering::SeqCst);

    println("WELCOME TO RUSTIC OS!");
    println("Rustic OS へようこそ!");
    println("UP TIME:");
    println(timer::get_uptime_ms().to_string().as_str());


    println("System Info");
    let mut brand_string = [0u8; 48];

    // CPUIDの 0x80000002, 0x80000003, 0x80000004 を順番に実行
    for i in 0u32..3u32 { // 明示的にu32で回す
        let res =__cpuid(0x80000002 + i);
        let registers = [res.eax, res.ebx, res.ecx, res.edx];

        for (j, &reg) in registers.iter().enumerate() {
            let bytes = reg.to_le_bytes();
            for k in 0..4 {
                // ここ！計算結果全体を () で囲って as usize にする
                let index = ((i * 16) + (j as u32 * 4) + k as u32) as usize;
                brand_string[index] = bytes[k];
            }
        }
    }


    let name = core::str::from_utf8(&brand_string).unwrap_or("Unknown CPU");
    println(name.trim());

    //print_usable_memory_stats(mmap);

    println("Starting DWM...");

    update_screen();

    unsafe{dwm::main::dwm_main(VRAM_PTR, WIDTH, HEIGHT);}

}
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { unsafe { asm!("hlt") } }
}