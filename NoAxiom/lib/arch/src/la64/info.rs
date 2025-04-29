#![allow(unused)]

use loongArch64::{
    cpu::{get_palen, get_valen},
    register::{crmd, dmw0, dmw1, pgd, pgdh, pgdl, prcfg1, prcfg2, prcfg3, pwch, pwcl},
};

use super::LA64;
use crate::{ArchAsm, ArchInfo};

impl ArchInfo for LA64 {
    const ARCH_NAME: &'static str = "loongarch64";
    fn arch_info_print() {
        let hartid = LA64::get_hartid();
        info!("[LA64] arch: loongarch64");
        info!("[LA64] hart id: {}", hartid);
        // let boot_start_top_0 = ptr::from_ref(&BOOT_STACK) as usize;
        // let boot_start_top_cur = boot_start_top_0 + KERNEL_STACK_SIZE * hartid;
        // info!(
        //     "[LA64] kernel_stack addr: [{:#x}, {:#x})",
        //     boot_start_top_cur,
        //     boot_start_top_cur + KERNEL_STACK_SIZE
        // );
        // info!(
        //     "[LA64] max_valen: {}, max_palen: {}",
        //     get_valen(),
        //     get_palen()
        // );
        info!("[LA64] crmd: {:?}", crmd::read(),);
        // info!("[LA64] dmw0: {:?}", dmw0::read());
        // info!("[LA64] dmw1: {:?}", dmw1::read());
        // info!("[LA64] prcfg1: {:?}", prcfg1::read());
        // info!("[LA64] prcfg2: {:#x}", prcfg2::read().raw());
        // info!("[LA64] prcfg3: {:#x}", prcfg3::read().raw());

        // let pwcl = pwcl::read();
        // let pwch = pwch::read();
        // let info = [
        //     (pwcl.ptbase(), pwcl.ptwidth()),
        //     (pwcl.dir1_base(), pwcl.dir1_width()),
        //     (pwcl.dir2_base(), pwcl.dir2_width()),
        //     (pwch.dir3_base(), pwch.dir3_width()),
        //     (pwch.dir4_base(), pwch.dir4_width()),
        // ];
        // info!("[LA64] pwcl: {:#x}", pwcl::read().raw());
        // info!("[LA64] pwch: {:#x}", pwch::read().raw());
        // for i in 0..5 {
        //     info!("[LA64] pwc[{}]: {:?}", i, info[i]);
        // }

        info!("[LA64] pgd: {:?}", pgd::read());
        info!("[LA64] pgdl: {:?}", pgdl::read());
        info!("[LA64] pgdh: {:?}", pgdh::read());

        // let save: usize;
        // unsafe { asm!("csrrd {}, 0x30", out(reg) save) };
        // info!("[LA64] SAVE: {:#x}", save);
    }
}
