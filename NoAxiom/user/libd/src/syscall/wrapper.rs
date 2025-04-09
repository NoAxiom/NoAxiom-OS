//! 提供给用户的系统调用封装

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use virtio_input_decoder::{DecodeType, Decoder};

use crate::syscall::utils::Dirent64;

use super::{
    syscall::*,
    utils::{FileMode, OpenFlags},
};

pub fn fork() -> isize {
    sys_fork()
}

pub fn execve(path: &str, args: &[*const u8], envp: &[*const u8]) -> isize {
    sys_execve(
        path.as_ptr(),
        args.as_ptr() as *const usize,
        envp.as_ptr() as *const usize,
    )
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf.as_ptr(), buf.len())
}

pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf.as_mut_ptr(), buf.len())
}

pub fn exit(exit_code: isize) -> ! {
    sys_exit(exit_code);
    loop {}
}

pub fn clone(f: fn(*const u8) -> isize, stack: *const u8, flags: i32, arg: *const u8) -> isize {
    sys_clone(f, stack, flags, arg)
}

pub fn yield_() -> isize {
    sys_sched_yield()
}

pub fn wait(pid: isize, exit_code: &mut usize) -> isize {
    sys_waitpid(pid, exit_code as *mut _)
}

pub fn chdir(path: &str) -> isize {
    sys_chdir(path.as_ptr())
}

pub fn getcwd(path: &mut [u8]) -> isize {
    sys_getcwd(path.as_mut_ptr(), path.len())
}

pub const AT_FDCWD: isize = -100isize;
pub fn open(name: &str, flag: OpenFlags) -> isize {
    sys_openat(
        AT_FDCWD,
        name.as_ptr(),
        flag.bits() as usize,
        FileMode::FMODE_RDWR.bits() as usize,
    )
}

pub fn getdents_len(fd: usize, buf: &mut [u8]) -> isize {
    sys_getdents(fd, buf.as_mut_ptr(), buf.len())
}

pub fn system_shutdown() -> ! {
    __system_shutdown();
    panic!("Shutdown failed!");
}

pub fn framebuffer() -> isize {
    sys_framebuffer()
}

pub fn framebuffer_flush() -> isize {
    sys_framebuffer_flush()
}

pub fn event_get() -> Option<InputEvent> {
    let raw_value = sys_event_get();
    if raw_value == 0 {
        None
    } else {
        Some((raw_value as u64).into())
    }
}

#[repr(C)]
pub struct InputEvent {
    pub event_type: u16,
    pub code: u16,
    pub value: u32,
}

impl From<u64> for InputEvent {
    fn from(mut v: u64) -> Self {
        let value = v as u32;
        v >>= 32;
        let code = v as u16;
        v >>= 16;
        let event_type = v as u16;
        Self {
            event_type,
            code,
            value,
        }
    }
}

impl InputEvent {
    pub fn decode(&self) -> Option<DecodeType> {
        Decoder::decode(
            self.event_type as usize,
            self.code as usize,
            self.value as usize,
        )
        .ok()
    }
}

pub fn listen(name: &str) -> isize {
    sys_listen(name.as_ptr())
}

pub fn connect(name: &str) -> isize {
    sys_connect(name.as_ptr())
}

pub fn getdents(fd: usize, buf: &mut [u8]) -> Option<Vec<String>> {
    // let total_len = sys_getdents(fd, buf.as_mut_ptr(), buf.len());
    // if total_len < 0 {
    //     return None;
    // }
    // let mut result: Vec<String> = Vec::new();
    // let mut index: usize = 0;
    // loop {
    //     if index >= total_len as usize {
    //         break;
    //     }
    //     let cstr = CStr::from_bytes_until_nul(&buf[index..]).unwrap();
    //     let str = cstr.to_string_lossy().to_string();
    //     index += str.len() + 1;
    //     result.push(str);
    // }
    let result = ls_prase2(fd,"./\0");
    Some(result)
}

pub fn close(fd: usize) -> isize {
    sys_close(fd)
}

// pub fn exec(path: &str) -> isize {
//     sys_execve(
//         path.as_ptr(),
//         &[core::ptr::null::<u8>()],
//         &[core::ptr::null::<u8>()],
//     )
// }

pub fn get_time() -> isize {
    sys_get_time()
}


const BUF_SIZE: usize = 1024;
pub fn ls_prase2(fd: usize,path: &str) ->Vec<String> {
    let mut result: Vec<String> = Vec::new();
    assert!(fd >= 0, "open failed");
    let mut buf = [0u8; BUF_SIZE];
    loop {
        let size = getdents_len(fd as usize, &mut buf);
        // assert!(size >= 0, "getdents failed");
        if size == 0 || size < 0 {
            break;
        }
        let mut ptr = buf.as_ptr();
        let mut count = 0;
        loop {
            let dirent = unsafe { &*(ptr as *const Dirent64) };
            if dirent.get_name() != "." && dirent.get_name() != ".." {
                // println!("{} {}", trans(dirent.type_), dirent.get_name());
                println!("{}", dirent.get_name());
                let name = dirent.get_name().to_string();
                result.push(name);
            }
            count += dirent.len();
            if count >= size as usize {
                break;
            }
            ptr = unsafe { ptr.add(dirent.len()) };
        }
        buf.fill(0);
    }
    result
}