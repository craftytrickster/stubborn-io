macro_rules! logc {
    ($lvl:expr, $($arg:tt)*) => {
	#[cfg(feature = "log")]
        log::log!($lvl, $($arg)*);
    };
}

pub(crate) use logc;
