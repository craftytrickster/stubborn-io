#[allow(unused_macros)]
macro_rules! trace {
    ($($arg:tt)*) => {
	#[cfg(feature = "log")]
        log::trace!($($arg)*);
    };
}

#[allow(unused_macros)]
macro_rules! debug {
    ($($arg:tt)*) => {
	#[cfg(feature = "log")]
        log::debug!($($arg)*);
    };
}

macro_rules! info {
    ($($arg:tt)*) => {
	#[cfg(feature = "log")]
        log::info!($($arg)*);
    };
}

#[allow(unused_macros)]
macro_rules! warn {
    ($($arg:tt)*) => {
	#[cfg(feature = "log")]
        log::warn!($($arg)*);
    };
}

macro_rules! error {
    ($($arg:tt)*) => {
	#[cfg(feature = "log")]
        log::error!($($arg)*);
    };
}

pub(crate) use {error, info};
