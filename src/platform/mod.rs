cfg_if::cfg_if! {
    if #[cfg(target_vendor = "apple")] {
        mod apple;
        pub use self::apple::*;
    } else if #[cfg(windows)] {
        mod windows;
        pub use self::windows::*;
    } else {
        mod unsupported;
        pub use self::unsupported::*;
    }
}
