# NoAxiom-OS

## 赛事相关

[原项目](https://github.com/NoAxiom/NoAxiom-OS)（包含submodule）

[初赛汇报](./docs/NoAxiom_OS_Primary_Report.pdf)

[初赛PPT与演示视频](https://pan.baidu.com/s/1aj0eP2t-oPZIlO7OO4S8pA?pwd=kkxz)

## 项目概述

### 系统简介

[NoAxiom 操作系统](https://github.com/NoAxiom/NoAxiom-OS)是由杭州电子科技大学[NoAxiom团队](https://github.com/NoAxiom)开发的一款基于 Rust 的宏内核操作系统，能够在 RISC-V64 和 LoongArch64 两种架构上运行。系统采用 Rust 的无栈协程与异步语法实现了**异步调度**，在 I/O 方面具备优秀性能。

### 系统整体架构

NoAxiom 操作系统整体分为以下四个层次：**机器层**、**硬件抽象层**、**内核实现层**、**用户层**

### 架构分层说明

#### 机器层（Machine Layer）

目标是支持多平台多架构（如 RISC-V64 与 LoongArch64），同时适配 QEMU 虚拟机与物理开发板平台。这些被统一归为机器层，并由上层的硬件抽象层统一封装。

#### 硬件抽象层（HAL, Hardware Abstraction Layer）

该层抽象底层硬件，向上提供统一接口，主要子模块如下：

1. **指令集架构抽象层**：定义统一 `Arch` trait，封装所有架构相关的函数与常量，位于 `lib/arch`。
2. **平台抽象层**：根据编译时平台定义常量，解耦虚拟与物理环境差异。
3. **内存抽象层**：封装架构/平台相关的内存初始化逻辑，提供统一访存接口。
4. **驱动抽象层**：启动时自动设备嗅探与注册，支持设备树与中断注册（如 RISC-V 下的 PLIC）。

#### 内核实现层（Kernel Implementation Layer）

基于 HAL 提供的接口，内核实现各类功能子模块，并向用户提供统一系统调用接口。主要子模块包括：

1. **进程管理**：维护 PCB、任务管理器等。
2. **任务调度**：基于无栈协程实现异步调度。
3. **文件系统**：通过 VFS 抽象底层，支持多文件系统、异步调度与 I/O 多路复用。
4. **信号系统**：支持进程间异步通信。
5. **内存管理**：支持懒分配、异常检查、用户指针校验。
6. **时间管理**：支持可靠的定时功能。
7. **网络模块**：实现 TCP/UDP、支持 IPv4/IPv6，具备高并发性能。

#### 用户应用层（User Application Layer）

提供用户程序运行支持，初始进程 ELF 文件内嵌于内核中，赛事测试样例运行于此层。

### 系统完成情况

截至目前，NoAxiom 实现了 **115 个系统调用**，覆盖以下功能领域：

* 文件系统
* IO
* 网络
* 进程管理
* 信号处理
* 内存管理
* 调度管理
* 时间管理

系统已成功运行大部分官方测例（除少数 ltp 测试点外）。具体子模块完成情况如下：

---

### NoAxiom 系统子模块完成情况

| **子模块**   | **实现情况**                                                                                                                              |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| **进程管理**  | - 统一的进程资源抽象<br>- **细粒度**共享资源                                                                                                          |
| **内存管理**  | - 内核与用户地址空间共享<br>- **懒分配与写时复制**<br>- 快速检查用户指针**合法性**<br>- 文件映射懒分配的完整**异步让权**                                                          |
| **文件系统**  | - 类 Linux 的 **VFS 虚拟文件系统**<br>- 支持管道、套接字等虚拟文件挂载<br>- 支持异步 EXT4、FAT32<br>- 耗时读写操作的**异步让权**<br>- 实现高效的**页缓存**<br>- 支持异步让权下的**I/O 多路复用** |
| **任务调度**  | - 完整的**分时多任务异步调度**<br>- 抽象统一调度器特性<br>- 支持**任务优先级**                                                                                    |
| **信号系统**  | - 实现信号系统维护<br>- 支持**可被信号中断**的系统调用                                                                                                     |
| **硬件抽象层** | - 自主支持 RISC-V64 和 LoongArch64<br>- 统一解耦的硬件接口设计<br>- 架构解耦的访存与中断机制                                                                      |
| **设备驱动**  | - 多架构下的**设备嗅探机制**<br>- 异步块设备驱动<br>- **异步块缓存**支持                                                                                       |
| **网络模块**  | - 支持 TCP/UDP 套接字<br>- 支持 IPv4/IPv6 协议<br>- 实现**端口复用**<br>- 支持等待过程的**异步让权**                                                            |

---

EN ver

## Tutorial

### Quick Start

Just run `make all`, and it will generate `kernel_rv` and `kernel_la` automatically.

```shell
make all
```

### Environment Setup

Before starting, you should set up the environment.

If you are not running on Linux, you may use `make docker` to create a virtual machine environment.

Run `make env` to automatically set up the environment. It will initialize Rust targets, vendor sources, and Git submodules.

Note: If you want to debug in vscode, use `make vscode` to quickly generate vscode settings.

Use `make help` and `make info` to get more detailed information.

### Build

Considering that our kernel supports multiple architectures, we provide a convenient way to build all variants: run `make build-all`, which will generate kernel binaries for all supported architectures and libraries.

To build a specific architecture and library, use `make build ARCH_NAME= LIB_NAME=`.

You can also use `make all` to set up the environment and build all targets. The binary and ELF files will be copied to `./output/`.

### Run

Currently, the kernel runs only on QEMU. Use `make run` to launch it with the default architecture and library. You can also specify them via `ARCH_NAME` and `LIB_NAME`.

Use `make default` or simply `make` to build and run.

The optional parameters are as follows:  

| Parameter | Description | Default | Optional |
| :--- | :--- | :--- | :--- |
| `TEST_TYPE` | The test suite to run.      | `custom`      | `official`、`custom`(when use `custom`, the `ARCH_NAME` and `LIB_NAME` will just be `riscv64` and `musl`, the parameters input will be ignored) |
| `ARCH_NAME` | The architecture to run.    | `riscv64`     | `riscv64`、`loongarch64` |
| `LIB_NAME`  | The library to run.         | `glibc`       | `glibc`、`musl` |
| `LOG`       | The log level.              | `info`        | `info`、`debug`、`warn`、`error` |

for example, to run the kernel with `loongarch64` and `musl` at `official`, use:

```shell
make TEST_TYPE=official ARCH_NAME=loongarch64 LIB_NAME=musl
```

## Thanks

| Project                                                      | Referred Function                |
| ------------------------------------------------------------ | -------------------------------- |
| [rCore](https://rcore-os.cn/rCore-Tutorial-Book-v3/index.html) | Tutorial                         |
| [Pantheon](https://gitee.com/LiLiangF/pantheon_visionfive)   | Coroutine, Process, Memory       |
| [DragonOS](https://github.com/DragonOS-Community/DragonOS)   | Net structure                    |
| [Phoenix](https://gitlab.eduxiji.net/educg-group-22026-2376550/T202418123993075-1053) | Coroutine, Memory, Signal        |
| [NPUcore-IMPACT](https://gitlab.eduxiji.net/educg-group-22027-2376549/T202410699992496-1562) | LoongArch support for arch       |
| [Tornado-OS](https://github.com/HUST-OS/tornado-os)          | Async driver for file system     |
| [Polyhal](https://github.com/Byte-OS/polyhal)                | Arch layer design for multi-arch |
