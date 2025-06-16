#[allow(unused)]
pub const NOAXIOM_BANNER_COLORFUL: &str = concat!(
    "\n",
    "\u{1B}[1;91m    _   __      ___         _               \u{1B}[0m\n",
    "\u{1B}[1;93m   / | / /___  /   |  _  __(_)___  ____ ___ \u{1B}[0m\n",
    "\u{1B}[1;92m  /  |/ / __ \\/ /| | | |/_/ / __ \\/ __ `__ \\\u{1B}[0m\n",
    "\u{1B}[1;96m / /|  / /_/ / ___ |_>  </ / /_/ / / / / / /\u{1B}[0m\n",
    "\u{1B}[1;94m/_/ |_/\\____/_/  |_/_/|_/_/\\____/_/ /_/ /_/ \u{1B}[0m\n"
);

#[allow(unused)]
pub const NOAXIOM_BANNER_MONOCHROME: &str = r#"
    _   __      ___         _               
   / | / /___  /   |  _  __(_)___  ____ ___ 
  /  |/ / __ \/ /| | | |/_/ / __ \/ __ `__ \
 / /|  / /_/ / ___ |_>  </ / /_/ / / / / / /
/_/ |_/\____/_/  |_/_/|_/_/\____/_/ /_/ /_/ 
"#;

#[cfg(feature = "debug_sig")]
pub use NOAXIOM_BANNER_COLORFUL as NOAXIOM_BANNER;
#[cfg(not(feature = "debug_sig"))]
pub use NOAXIOM_BANNER_MONOCHROME as NOAXIOM_BANNER;
