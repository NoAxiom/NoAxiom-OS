# NoAxiom-OS

## Tutorial

### Quick Start

Just run `make all`, and see the output in [`./output/`](./output/)

```shell
make all
```

### Environment Setup

Before starting, you should set up the environment.

If you are not running on Linux, you may use `make docker` to create a virtual machine environment.

Run `make env` to automatically set up the environment. It will initialize Rust targets, vendor sources, and Git submodules.

Note: If you want to debug in VSCode, use `make vscode` to quickly generate VSCode settings.

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

[Pantheon](https://gitee.com/LiLiangF/pantheon_visionfive)

[rCore](https://rcore-os.cn/rCore-Tutorial-Book-v3/index.html)

[DragonOS](https://github.com/DragonOS-Community/DragonOS)

[Titanix](https://gitlab.eduxiji.net/202318123101314/oskernel2023-Titanix)

[Phoenix](https://gitlab.eduxiji.net/educg-group-22026-2376550/T202418123993075-1053)
