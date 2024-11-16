//! NoAxiom global configs
//! provides constants for kernel config
//! [`arch`] contains configs for architecture and cpus
//! [`mm`] contains configs for memory management
//! [`sched`] contains configs for task / coroutine schedule

/// NoAxiom banner for kernel init message
pub const NOAXIOM_BANNER: &str = r#"
    _   __      ___         _               
   / | / /___  /   |  _  __(_)___  ____ ___ 
  /  |/ / __ \/ /| | | |/_/ / __ \/ __ `__ \
 / /|  / /_/ / ___ |_>  </ / /_/ / / / / / /
/_/ |_/\____/_/  |_/_/|_/_/\____/_/ /_/ /_/ 
"#;

pub mod arch;
pub mod mm;
pub mod sched;
