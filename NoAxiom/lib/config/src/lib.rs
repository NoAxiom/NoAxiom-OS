//! NoAxiom Configs
//! this lib provides constants for kernel config

#![no_std]
#![no_main]

/// NoAxiom banner for kernel init message
pub const NOAXIOM_BANNER: &str = r#"
    _   __      ___         _               
   / | / /___  /   |  _  __(_)___  ____ ___ 
  /  |/ / __ \/ /| | | |/_/ / __ \/ __ `__ \
 / /|  / /_/ / ___ |_>  </ / /_/ / / / / / /
/_/ |_/\____/_/  |_/_/|_/_/\____/_/ /_/ /_/ 
"#;

/// CPU count for multi-core
pub const CPU_NUM: usize = 2;


/// kernel heap size for heap allocator
pub const KERNEL_HEAP_SIZE: usize = 0x200_0000;