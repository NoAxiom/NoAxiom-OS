use alloc::boxed::Box;

use super::{file::File, FileReturn};
use crate::task::load_app::{app_nums, get_app_data};

const MAX_APP_NUMS: usize = 100;

lazy_static::lazy_static! {
    static ref APPS: [TmpFile; MAX_APP_NUMS] = {
        let mut apps = [TmpFile { app_id: 0 }; MAX_APP_NUMS];
        for i in 0..app_nums() {
            apps[i] = TmpFile { app_id: i };
        }
        apps
    };
}

pub fn get_app_elf(app_id: usize) -> TmpFile {
    APPS[app_id]
}

#[derive(Debug, Copy, Clone)]
pub struct TmpFile {
    app_id: usize,
}

impl File for TmpFile {
    fn read<'a>(&'a self, addr: usize, len: usize, buf: &'a mut [u8]) -> FileReturn {
        Box::pin(async move {
            let data = get_app_data(self.app_id);
            buf[0..len].copy_from_slice(&data[addr..addr + len]);
            info!("addr: {}, len: {}, total_len: {}", addr, len, data.len());
            assert!(addr == 0 && len <= data.len());
            Ok(len as isize)
        })
    }

    fn write<'a>(&'a self, addr: usize, buf: &'a [u8]) -> FileReturn {
        let _ = addr;
        let _ = buf;
        Box::pin(async { Ok(buf.len() as isize) })
    }

    fn flush(&self) -> Result<(), ()> {
        Err(())
    }

    fn close(&self) -> Result<(), ()> {
        Err(())
    }
}
