use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
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

pub struct StatusFile {
    meta: FileMeta,
    // meminfo: Arc<StatusInfo>,
}

impl StatusFile {
    pub fn new(meta: FileMeta) -> Self {
        Self {
            meta,
            // meminfo: Arc::new(StatusInfo::default()),
        }
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
impl File for StatusFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    // todo: /proc/self/status isn't well implemented
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        let path = self.meta.dentry().path();
        let tid = resolve_path_tid(&path);
        let task = match tid {
            StatTid::SelfTid => current_task().unwrap(),
            StatTid::Tid(x) => &TASK_MANAGER.get(x).ok_or_else(|| {
                error!("[StatusFile::base_read] Failed to get task, return EINVAL");
                Errno::EINVAL
            })?,
        };
        // todo: maybe can just read empty

        // 名称：这里我们用执行路径的文件名部分作为任务名（类似 bash）
        let name = task.exe().clone();
        let umask = 0o022; // 默认umask为022(fake)
        let state_str = "R (running)";
        let tgid = task.tgid();
        let ngid = 0; // NUMA 组 ID（如果没有则为 0）
        let pid = task.pid();
        let ppid = task.pcb().parent.as_ref().unwrap().upgrade().unwrap().tid();

        let tracerpid = 0; // 跟踪此进程的进程 PID（如果未被跟踪，则为 0）
        let uid = task.uid();
        let euid = task.euid();
        let suid = task.suid();
        let fsuid = task.fsuid();
        let gid = task.gid();
        let egid = task.egid();
        let sgid = task.sgid();
        let fsgid = task.fsgid();
        let fdsize = task.fd_table().rlimit().rlim_cur as usize;
        let groups = String::new();
        let nstgid = task.tgid(); // pid 所属的每个 PID 命名空间中的线程组 ID
        let nstpid = task.tid(); // pid 所属的每个 PID 命名空间中的线程 ID
        let nspgid = task.get_pgid(); // pid 所属的每个 PID 命名空间中的进程组 ID
        let nssid = task.tgid(); // pid 所属的每个 PID 命名空间中的会话 ID
        let vmpeak = 3356; // 虚拟内存峰值（fake）需要遍历统计
        let vmsize = 3356; // 虚拟内存大小（fake）
        let vmlck = 0; // 锁定的虚拟内存大小（fake）
        let vmpin = 0; // 锁定的物理内存大小（fake）
        let vmhwm = 1076; // 常驻内存峰值（fake）
        let vmrss = 1076; // 常驻内存大小（fake）请注意，此处的值是 RssAnon、RssFile 和 RssShmem 的总和
        let rssanon = 92; // 匿名内存（fake）
        let rssfile = 984; // 文件映射的常驻内存（fake）
        let rssshmem = 0; // 共享内存的常驻内存（fake）
        let vmdata = 3840; // 数据段大小（fake）
        let vmstk = 2570; // 栈大小（fake）
        let vmexe = 378; // 可执行文件大小（fake）
        let vmlib = 993; // 共享库大小（fake）
        let vmpte = 85; // 页表大小（fake）
        let vmswap = 169; // 交换空间大小（fake）
        let hugetlbpages = 0; // 巨页内存大小（fake）
        let core_dumping = 0; // 核心转储大小（fake）
        let thp_enabled = 1; // 透明大页是否启用（fake）
                             // let threads = self.op_thread_group(|tg| tg.len());
        let threads = 0;
        let sigq = 1; // 信号队列大小（fake）
        let sigpnd = task.sig_mask(); // 信号掩码
        let shdpnd = 0; // 共享信号掩码（fake）
        let sigblk = 0; // 阻塞的信号掩码（fake）
        let sigign = 0; // 忽略的信号掩码（fake）
        let sigcatch = 0; // 捕获的信号掩码（fake）
        let cap_inheritable = 0; // 可继承的能力（fake）
        let cap_permitted = 0; // 允许的能力（fake）
        let cap_effective = 0; // 有效的能力（fake）
        let cap_bounding = 0x000001ffffffffff as i64; // 边界能力（fake）
        let cap_ambient = 0; // 环境能力（fake）
        let no_new_privs = 0; // 是否设置了 no_new_privs（fake）
        let seccomp = 0; // seccomp 状态（fake）
        let seccomp_filter = 0; // seccomp 过滤器（fake）
        let speculation_store_bypass = "thread vulnerable".to_string();
        let speculation_indirect_branch = "conditional enabled".to_string();
        let cpus_allowed = 1; // 允许的 CPU 掩码（fake）
        let cpus_allowed_list = "0".to_string(); // 允许的 CPU 列表（fake）
        let mems_allowed = 1; // 允许的内存节点掩码（fake）
        let mems_allowed_list = "0".to_string(); // 允许的内存节点列表（fake）
        let voluntary_ctxt_switches = 0; // 自愿上下文切换次数（fake）
        let nonvoluntary_ctxt_switches = 0; // 非自愿上下文切换次数（fake）

        // 构造信息
        let proc_info = format!(
            "\
            Name:\t{}\n\
            Umask:\t{:04o}\n\
            State:\t{}\n\
            Tgid:\t{}\n\
            Ngid:\t{}\n\
            Pid:\t{}\n\
            PPid:\t{}\n\
            TracerPid:\t{}\n\
            Uid:\t{}\t{}\t{}\t{}\n\
            Gid:\t{}\t{}\t{}\t{}\n\
            FDSize:\t{}\n\
            Groups:\t{}\n\
            NStgid:\t{}\n\
            NSpid:\t{}\n\
            NSpgid:\t{}\n\
            NSsid:\t{}\n\
            VmPeak:\t{:>8} kB\n\
            VmSize:\t{:>8} kB\n\
            VmLck:\t{:>8} kB\n\
            VmPin:\t{:>8} kB\n\
            VmHWM:\t{:>8} kB\n\
            VmRSS:\t{:>8} kB\n\
            RssAnon:\t{:>8} kB\n\
            RssFile:\t{:>8} kB\n\
            RssShmem:\t{:>8} kB\n\
            VmData:\t{:>8} kB\n\
            VmStk:\t{:>8} kB\n\
            VmExe:\t{:>8} kB\n\
            VmLib:\t{:>8} kB\n\
            VmPTE:\t{:>8} kB\n\
            VmSwap:\t{:>8} kB\n\
            HugetlbPages:\t{:>8} kB\n\
            CoreDumping:\t{}\n\
            THP_enabled:\t{}\n\
            Threads:\t{}\n\
            SigQ:\t{}/31760\n\
            SigPnd:\t{:016x}\n\
            ShdPnd:\t{:016x}\n\
            SigBlk:\t{:016x}\n\
            SigIgn:\t{:016x}\n\
            SigCgt:\t{:016x}\n\
            CapInh:\t{:016x}\n\
            CapPrm:\t{:016x}\n\
            CapEff:\t{:016x}\n\
            CapBnd:\t{:016x}\n\
            CapAmb:\t{:016x}\n\
            NoNewPrivs:\t{}\n\
            Seccomp:\t{}\n\
            Seccomp_filters:\t{}\n\
            Speculation_Store_Bypass:\t{}\n\
            SpeculationIndirectBranch:\t{}\n\
            Cpus_allowed:\t{:x}\n\
            Cpus_allowed_list:\t{}\n\
            Mems_allowed:\t{:x}\n\
            Mems_allowed_list:\t{}\n\
            voluntary_ctxt_switches:\t{}\n\
            nonvoluntary_ctxt_switches:\t{}\n",
            name,
            umask,
            state_str,
            tgid,
            ngid,
            pid,
            ppid,
            tracerpid,
            uid,
            euid,
            suid,
            fsuid,
            gid,
            egid,
            sgid,
            fsgid,
            fdsize,
            groups.trim_end(),
            nstgid,
            nstpid,
            nspgid,
            nssid,
            vmpeak,
            vmsize,
            vmlck,
            vmpin,
            vmhwm,
            vmrss,
            rssanon,
            rssfile,
            rssshmem,
            vmdata,
            vmstk,
            vmexe,
            vmlib,
            vmpte,
            vmswap,
            hugetlbpages,
            core_dumping,
            thp_enabled,
            threads,
            sigq,
            sigpnd,
            shdpnd,
            sigblk,
            sigign,
            sigcatch,
            cap_inheritable,
            cap_permitted,
            cap_effective,
            cap_bounding,
            cap_ambient,
            no_new_privs,
            seccomp,
            seccomp_filter,
            speculation_store_bypass,
            speculation_indirect_branch,
            cpus_allowed,
            cpus_allowed_list,
            mems_allowed,
            mems_allowed_list,
            voluntary_ctxt_switches,
            nonvoluntary_ctxt_switches,
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
