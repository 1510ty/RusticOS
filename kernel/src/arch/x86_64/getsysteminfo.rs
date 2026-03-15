use alloc::string::{String, ToString};
use core::arch::x86_64::__cpuid;
use crate::draw::println;

pub fn get_cpu_name() -> String {
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
    name.trim().to_string()
}