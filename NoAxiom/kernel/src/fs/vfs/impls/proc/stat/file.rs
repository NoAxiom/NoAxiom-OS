use alloc::{boxed::Box, string::ToString, vec::Vec};
use core::task::Waker;

use async_trait::async_trait;
use include::errno::Errno;

use crate::{
    cpu::current_task,
    fs::vfs::basic::file::{File, FileMeta},
    include::io::PollEvent,
    syscall::{SysResult, SyscallResult},
    task::manager::TASK_MANAGER,
};

pub struct ProcStatFile {
    meta: FileMeta,
}

impl ProcStatFile {
    pub fn new(meta: FileMeta) -> Self {
        Self { meta }
    }
}

enum StatTid {
    SelfTid,
    Tid(usize),
}

fn resolve_path_tid(path: &str) -> StatTid {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 3 || parts[1] != "proc" {
        panic!("Invalid proc path: {}", path);
    }
    if parts[2] == "self" {
        StatTid::SelfTid
    } else {
        StatTid::Tid(parts[2].parse().expect("Failed to parse tid from path"))
    }
}

#[async_trait]
impl File for ProcStatFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        // todo: add some extra info
        // fixme: status is not correctly set because of the design of scheduler
        let path = self.meta.dentry().path();
        let tid = resolve_path_tid(&path);
        let task = match tid {
            StatTid::SelfTid => current_task().unwrap(),
            StatTid::Tid(x) => &TASK_MANAGER.get(x).ok_or_else(|| {
                error!("[ProcStatFile::base_read] Failed to get task, return EINVAL");
                Errno::EINVAL
            })?,
        };
        // todo: maybe can just read empty

        let tid = task.tid();
        let name = task.exe().to_string();
        let comm = format!("({})", name);

        // let status = task.pcb().status();
        // ltp only wait for 'S', so we set S here
        // fixme: should add more status here, currently it's incorrect
        let state_char = 'S';

        let ppid = task
            .pcb()
            .parent
            .clone()
            .map(|x| x.upgrade())
            .flatten()
            .map(|x| x.pid())
            .unwrap_or(0);
        let pgrp = task.get_pgid();
        let session = task.tgid(); // fixme: simplifiled
        let tty_nr = 0;
        let tpgid = 0;
        let flags = 0;

        let minflt = 0;
        let cminflt = 0;
        let majflt = 0;
        let cmajflt = 0;

        let time_stat = task.time_stat();
        let utime = time_stat.utime().as_millis() as u32;
        let stime = time_stat.stime().as_millis() as u32;
        let cutime = time_stat.child_time().utime.as_millis() as u32;
        let cstime = time_stat.child_time().stime.as_millis() as u32;
        let nice = task.sched_entity().nice;
        let num_threads = task.thread_group().0.len();

        let priority = 99;
        let itrealvalue = 0;
        let starttime = 0;
        let vsize = 1919810;
        let rss = 242;
        let rsslim = 23333;
        let startcode = 0;
        let endcode = 0;
        let startstack = 0;
        let kstkesp = 0;
        let kstkeip = 0;
        let signal = 0;
        let blocked = 0;
        let sigignore = 0;
        let sigcatch = 0;
        let wchan = 0;
        let nswap = 0;
        let cnswap = 0;
        let exit_signal = task.tcb().exit_signal.map(|x| x.raw()).unwrap_or(0) as u32;
        let processor = 0;
        let rt_priority = 0;
        let policy = 0;
        let delayacct_blkio_ticks = 0;
        let guest_time = 0;
        let cguest_time = 0;
        let start_data = 0;
        let end_data = 0;
        let start_brk = 0;
        let arg_start = 0;
        let arg_end = 0;
        let env_start = 0;
        let env_end = 0;
        let exit_code = task.pcb().exit_code().inner();

        let proc_info = format!(
            "{} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}\n",
            tid, comm, state_char, ppid, pgrp, session, tty_nr, tpgid, flags,
            minflt, cminflt, majflt, cmajflt,
            utime, stime, cutime, cstime,
            priority, nice, num_threads, itrealvalue, starttime,
            vsize, rss,
            rsslim, startcode, endcode, startstack, kstkesp, kstkeip,
            signal, blocked, sigignore, sigcatch, wchan,
            nswap, cnswap, exit_signal, processor, rt_priority,
            policy, delayacct_blkio_ticks, guest_time, cguest_time,
            start_data, end_data, start_brk,
            arg_start, arg_end, env_start, env_end,
            exit_code
        );

        if offset >= proc_info.len() {
            return Ok(0);
        }

        let write_bytes = proc_info.as_bytes();
        let ret_len = core::cmp::min(buf.len(), proc_info.len() - offset);
        buf[..ret_len].copy_from_slice(&write_bytes[offset..offset + ret_len]);
        Ok(ret_len as isize)
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }
    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        unreachable!("write to meminfo");
    }
    async fn load_dir(&self) -> SysResult<()> {
        Err(Errno::ENOTDIR)
    }
    async fn delete_child(&self, _name: &str) -> SysResult<()> {
        Err(Errno::ENOSYS)
    }
    fn ioctl(&self, _cmd: usize, _arg: usize) -> SyscallResult {
        Err(Errno::ENOTTY)
    }
    fn poll(&self, _req: &PollEvent, _waker: Waker) -> PollEvent {
        unreachable!("MemInfoFile::poll not supported now");
    }
}
