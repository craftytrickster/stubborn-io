#[allow(unused_macros)]
macro_rules! trace {
    ($($arg:expr),*) => {
	#[cfg(feature = "log")]
        log::trace!($($arg),*);
	#[cfg(not(feature = "log"))]
	{
	    $(
		let _ = $arg;
	    )*
	}
    };
}

#[allow(unused_macros)]
macro_rules! debug {
    ($($arg:expr),*) => {
	#[cfg(feature = "log")]
        log::debug!($($arg),*);
	#[cfg(not(feature = "log"))]
	{
	    $(
		let _ = $arg;
	    )*
	}
    };
}

macro_rules! info {
    ($($arg:expr),*) => {
	#[cfg(feature = "log")]
        log::info!($($arg),*);
	#[cfg(not(feature = "log"))]
	{
	    $(
		let _ = $arg;
	    )*
	}
    };
}

#[allow(unused_macros)]
macro_rules! warn {
    ($($arg:expr),*) => {
	#[cfg(feature = "log")]
        log::warn!($($arg),*);
	#[cfg(not(feature = "log"))]
	{
	    $(
		let _ = $arg;
	    )*
	}
    };
}

macro_rules! error {
    ($($arg:expr),*) => {
	#[cfg(feature = "log")]
        log::error!($($arg),*);
	#[cfg(not(feature = "log"))]
	{
	    $(
		let _ = $arg;
	    )*
	}
    };
}

pub(crate) use {error, info};
