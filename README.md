# NoAxiom-OS

```s
.
├── Cargo.lock
├── Cargo.toml
├── doc
│   └── 命名规范.md
├── Makefile
├── others
│   ├── bootloader
│   │   └── rustsbi-qemu.bin
│   └── linker_script
│       └── linker.ld
├── README.md
└── src
    ├── entry
    │   ├── entry.asm
    │   └── mod.rs
    ├── language_items
    │   └── mod.rs
    ├── main.rs
    ├── sbi
    │   ├── consts.rs
    │   └── mod.rs
    ├── syscall
    │   └── mod.rs
    └── utils
        ├── mod.rs
        └── print_macro.rs

10 directories, 16 files
      17 text files.
      17 unique files.                              
       4 files ignored.

github.com/AlDanial/cloc v 1.90  T=0.01 s (2115.4 files/s, 46388.5 lines/s)
-------------------------------------------------------------------------------
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                             8             18              6            109
Markdown                         2             31              0             57
make                             1              8              0             45
TOML                             2              3              0             18
Assembly                         1              1              0             11
-------------------------------------------------------------------------------
SUM:                            14             61              6            240
-------------------------------------------------------------------------------
```