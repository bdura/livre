#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        #[cfg(feature = "debug")]
        eprintln!($($arg)*);
    }};
}
