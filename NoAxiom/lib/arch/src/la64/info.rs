#![allow(unused)]

use core::ptr;

use config::mm::KERNEL_STACK_SIZE;
use loongArch64::{
    cpu::*,
    register::{crmd, dmw0, dmw1, eentry, pgd, pgdh, pgdl, prcfg1, prcfg2, prcfg3, pwch, pwcl},
};

use super::LA64;
use crate::{la64::boot::BOOT_STACK, ArchAsm, ArchInfo};

impl ArchInfo for LA64 {
    const ARCH_NAME: &'static str = "loongarch64";
    fn arch_info_print() {
        let hartid = LA64::get_hartid();
        log::info!("[LA64] arch: loongarch64");
        log::info!("[LA64] hart id: {}", hartid);
        let boot_start_top_0 = ptr::from_ref(&BOOT_STACK) as usize;
        let boot_start_top_cur = boot_start_top_0 + KERNEL_STACK_SIZE * hartid;
        log::info!(
            "[LA64] kernel_stack addr: [{:#x}, {:#x})",
            boot_start_top_cur,
            boot_start_top_cur + KERNEL_STACK_SIZE
        );
        log::info!(
            "[LA64] max_valen: {}, max_palen: {}",
            get_valen(),
            get_palen()
        );
        log::info!("[LA64] crmd: {:?}", crmd::read(),);
        log::info!("[LA64] dmw0: {:?}", dmw0::read());
        log::info!("[LA64] dmw1: {:?}", dmw1::read());
        log::info!("[LA64] prcfg1: {:?}", prcfg1::read());
        log::info!("[LA64] prcfg2: {:#x}", prcfg2::read().raw());

        log::info!("[LA64] prid: {:#x}", get_prid());
        log::info!("[LA64] arch: {}", get_arch());
        log::info!("[LA64] palen (physical address bits): {}", get_palen());
        log::info!("[LA64] valen (virtual address bits): {}", get_valen());
        log::info!("[LA64] mmu support page: {}", get_mmu_support_page());
        log::info!("[LA64] iocsr supported: {}", get_support_iocsr());
        log::info!("[LA64] unaligned access supported (UAL): {}", get_ual());
        log::info!(
            "[LA64] read forbid supported: {}",
            get_support_read_forbid()
        );
        log::info!(
            "[LA64] execution protection supported: {}",
            get_support_execution_protection()
        );
        log::info!("[LA64] RPLV supported: {}", get_support_rplv());
        log::info!("[LA64] huge page supported: {}", get_support_huge_page());
        log::info!("[LA64] RVA supported: {}", get_support_rva());
        log::info!("[LA64] RVA length: {}", get_support_rva_len());
        log::info!("[LA64] LSPW supported: {}", get_support_lspw());

        let prcfg3 = prcfg3::read();
        let prcfg3_raw = prcfg3.raw();
        let prcfg3_mtlb_entries = prcfg3.mtlb_entries();
        let prcfg3_sltb_sets = prcfg3.sltb_sets();
        let prcfg3_stlb_ways = prcfg3.stlb_ways();
        let prcfg3_tlb_type = prcfg3.tlb_type();
        log::info!(
            "[LA64] prcfg3: {:#x}, tlb_type: {}, mtlb_entries: {}, sltb_sets: {}, stlb_ways: {}",
            prcfg3_raw,
            prcfg3_tlb_type,
            prcfg3_mtlb_entries,
            prcfg3_sltb_sets,
            prcfg3_stlb_ways
        );

        let pwcl = pwcl::read();
        let pwch = pwch::read();
        let info = [
            (pwcl.ptbase(), pwcl.ptwidth()),
            (pwcl.dir1_base(), pwcl.dir1_width()),
            (pwcl.dir2_base(), pwcl.dir2_width()),
            (pwch.dir3_base(), pwch.dir3_width()),
            (pwch.dir4_base(), pwch.dir4_width()),
        ];
        log::info!("[LA64] pwcl: {:#x}", pwcl::read().raw());
        log::info!("[LA64] pwch: {:#x}", pwch::read().raw());
        for i in 0..5 {
            log::info!("[LA64] pwc[{}]: {:?}", i, info[i]);
        }

        log::info!("[LA64] pgd: {:?}", pgd::read());
        log::info!("[LA64] pgdl: {:?}", pgdl::read());
        log::info!("[LA64] pgdh: {:?}", pgdh::read());

        log::info!("[LA64] save num: {}", prcfg1::read().save_num());

        extern "C" {
            fn __kernel_trapvec();
        }
        let eentry = eentry::read().eentry();
        log::info!("[LA64] eentry: {:#x}, real: {:#x}", eentry, __kernel_trapvec as usize);
    }
}
