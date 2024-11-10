#![no_std]
#![no_main]
#![allow(unused)]

use core::arch::asm;

use thiserror::Error;

/// Completed successfully
pub const SBI_SUCCESS: isize = 0;
/// Failed
pub const SBI_ERR_FAILED: isize = -1;
/// Not supported
pub const SBI_ERR_NOT_SUPPORTED: isize = -2;
/// Invalid parameter(s)
pub const SBI_ERR_INVALID_PARAM: isize = -3;
/// Denied or not allowed
pub const SBI_ERR_DENIED: isize = -4;
/// Invalid address(s)
pub const SBI_ERR_INVALID_ADDRESS: isize = -5;
/// Already available
pub const SBI_ERR_ALREADY_AVAILABLE: isize = -6;
/// Already started
pub const SBI_ERR_ALREADY_STARTED: isize = -7;
/// Already stopped
pub const SBI_ERR_ALREADY_STOPPED: isize = -8;
/// Shared memory not available
pub const SBI_ERR_NO_SHMEM: isize = -9;

/// SBI 基本扩展
pub const EXTENSION_BASE: usize = 0x10;
/// SBI 时钟扩展
pub const EXTENSION_TIMER: usize = 0x54494D45;
// pub const EXTENSION_IPI: usize = 0x735049;
// pub const EXTENSION_RFENCE: usize = 0x52464E43;
/// SBI HSM 扩展
pub const EXTENSION_HSM: usize = 0x48534D;
// pub const EXTENSION_SRST: usize = 0x53525354;

/// SBI HSM扩展的启动cpu功能
const FUNCTION_HSM_HART_START: usize = 0x0;
// const FUNCTION_HSM_HART_STOP: usize = 0x1;
// const FUNCTION_HSM_HART_GET_STATUS: usize = 0x2;
const FUNCTION_HSM_HART_SUSPEND: usize = 0x3;

/// sbi调用返回值
#[repr(C)]
#[derive(Debug)]
pub struct SBIRet {
    pub error: isize,
    pub value: isize,
}

impl SBIRet {
    pub const fn new() -> Self {
        Self {
            error: isize::MIN,
            value: -1,
        }
    }
    pub fn get_sbi_error(&self) -> SBIError {
        self.error.into()
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SBIError {
    #[error("Completed successfully")]
    Success = SBI_SUCCESS,
    #[error("Failed")]
    Failed = SBI_ERR_FAILED,
    #[error("Not supported")]
    NotSupported = SBI_ERR_NOT_SUPPORTED,
    #[error("Invalid parameter(s)")]
    InvalidParam = SBI_ERR_INVALID_PARAM,
    #[error("Denied or not allowed")]
    Denied = SBI_ERR_DENIED,
    #[error("Invalid address(s)")]
    InvalidAddress = SBI_ERR_INVALID_ADDRESS,
    #[error("Already available")]
    AlreadyAvailable = SBI_ERR_ALREADY_AVAILABLE,
    #[error("Already started")]
    AlreadyStarted = SBI_ERR_ALREADY_STARTED,
    #[error("Already stopped")]
    AlreadyStopped = SBI_ERR_ALREADY_STOPPED,
    #[error("Shared memory not available")]
    NoShmem = SBI_ERR_NO_SHMEM,
}

impl From<isize> for SBIError {
    fn from(value: isize) -> Self {
        match value {
            0 => SBIError::Success,
            -1 => SBIError::Failed,
            -2 => SBIError::NotSupported,
            -3 => SBIError::InvalidParam,
            -4 => SBIError::Denied,
            -5 => SBIError::InvalidAddress,
            -6 => SBIError::AlreadyAvailable,
            -7 => SBIError::AlreadyStarted,
            -8 => SBIError::AlreadyStopped,
            -9 => SBIError::NoShmem,
            _ => panic!("invalid value of SBIError"),
        }
    }
}

#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => ret,
            in("a1") arg1,
            in("a2") arg2,
            in("a7") which,
        );
    }

    ret
}

#[inline(always)]
fn sbi_call_3(extension: usize, function: usize, arg0: usize, arg1: usize, arg2: usize) -> SBIRet {
    let (error, value);
    unsafe {
        asm!(
        "ecall",
        in("a0") arg0, in("a1") arg1, in("a2") arg2,
        in("a6") function, in("a7") extension,
        lateout("a0") error, lateout("a1") value,
        )
    }
    SBIRet { error, value }
}

// ===== legacy SBI call =====

const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_SHUTDOWN: usize = 8;

#[inline(always)]
fn legacy_sbi_call(eid: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    let mut ret;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => ret,
            in("a1") arg1,
            in("a2") arg2,
            in("a7") eid,
        );
    }
    ret
}

#[cfg_attr(
    target_arch = "riscv32",
    warn(
        deprecated,
        reason = "SBI v0.1 spec counld not been found, current implementation may not work on rv32"
    )
)]
pub fn set_timer(stime_value: u64) -> isize {
    #[cfg(target_arch = "riscv64")]
    {
        legacy_sbi_call(SBI_SET_TIMER, stime_value as usize, 0, 0)
    }
}

pub fn console_putchar(c: i32) -> isize {
    legacy_sbi_call(SBI_CONSOLE_PUTCHAR, c as usize, 0, 0)
}

pub fn console_getchar() -> isize {
    legacy_sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0)
}

pub fn shutdown() -> ! {
    legacy_sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    unreachable!("kernel has already shutdown");
}

/// wrap sbi FUNCTION_HSM_HART_START call
pub fn hart_start(hart_id: usize, start_addr: usize, opaque: usize) -> SBIRet {
    sbi_call_3(
        EXTENSION_HSM,
        FUNCTION_HSM_HART_START,
        hart_id,
        start_addr,
        opaque,
    )
}
