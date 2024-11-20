pub const DEBUG: bool = true;

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        if crate::commons::DEBUG {
            println!($($arg)*);
        }
    };
}