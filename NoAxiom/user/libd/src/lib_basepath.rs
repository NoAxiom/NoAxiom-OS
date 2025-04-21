#[cfg(feature = "glibc")]
mod lib_def {
    pub const LIB_NAME: &str = "glibc";
    pub const ROOT_NAME: &str = "/glibc";
}

#[cfg(feature = "musl")]
mod lib_def {
    pub const LIB_NAME: &str = "musl";
    pub const ROOT_NAME: &str = "/musl";
}

#[cfg(not(any(feature = "glibc", feature = "musl")))]
compile_error!(
    "Please select an valid lib implementation by enabling either the 'glibc' or 'musl' feature."
);

pub use lib_def::*;
