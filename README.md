# NoAxiom-OS

## Run

Check your environment:

```shell
make env
```

Run this manually if detect `vendor/` isn't under current directory:

```shell
make vendor
```

Run NoAxiom-OS:

```shell
make clean
make run
```

## File structure

Temporarily using [Pantheon](https://gitee.com/LiLiangF/pantheon_visionfive) project structure.

Plan to re-structure later.

## Toolchain

View `./rust-toolchain.toml` for further infomation.

Basic information is listed as below.

 - rust version: `nightly-2024-09-15`

 - rust target `riscv64gc-unknown-none-elf`
