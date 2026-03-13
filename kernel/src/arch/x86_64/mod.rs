use limine::response::ExecutableAddressResponse;

// kernel/src/arch/x86_64/mod.rs
pub mod gdt;
pub mod idt;
pub mod apic;
pub mod timer;

pub fn init(response: &ExecutableAddressResponse) {

    // 差分だけを計算しておく
    // 物理アドレス = 仮想アドレス + offset
    let offset = response.physical_base() as i64 - response.virtual_base() as i64;

    // GDTとIDTにこのオフセットを渡して初期化！
    gdt::init(offset);
    idt::init(offset);
}