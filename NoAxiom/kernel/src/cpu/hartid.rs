// use alloc::vec::Vec;
// use core::intrinsics::atomic_store_seqcst;

// use arch::{Arch, ArchAsm};
// use config::cpu::CPU_NUM;

// static mut HART_ID_MASK: [bool; CPU_NUM] = [false; CPU_NUM];

// pub unsafe fn register_hartid() {
//     atomic_store_seqcst(&mut HART_ID_MASK[Arch::get_hartid()] as *mut bool, true);
// }

// pub fn get_cpumask() -> [bool; CPU_NUM] {
//     unsafe { HART_ID_MASK }
// }

// pub fn get_valid_hartlist() -> Vec<usize> {
//     let mut hartlist = Vec::new();
//     for (i, &valid) in unsafe { HART_ID_MASK.iter() }.enumerate() {
//         if valid {
//             hartlist.push(i);
//         }
//     }
//     hartlist
// }
