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
| `LOG`       | The log level.              | `info`        | `info`、`debug`、`warn`、`error` |

for example, to run the kernel with `loongarch64` at `official`, use:

```shell
make TEST_TYPE=official ARCH_NAME=loongarch64
```
