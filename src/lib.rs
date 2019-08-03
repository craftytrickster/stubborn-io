#![feature(async_await)]
//! Contains the ingredients needed to create wrappers over tokio AsyncRead/AsyncWrite items
//! to automatically reconnect upon failures. This is done so that user can use them without worrying
//! that their application logic will terminate simply due to an event like a temporary network failure.
//!
//! This crate will try to provide commonly used io items, for example, the [StubbornTcpStream](StubbornTcpStream).
//! If you need to create your own, you simply need to implement the [UnderlyingIo](crate::tokio::UnderlyingIo) trait.
//! Once implemented, can construct it easily by creating a [StubbornIo](crate::tokio::StubbornIo) type as seen below.
//!
//! ## Example on how a Stubborn IO item might be created
//! ``` ignore
//! #![feature(async_await)]
//!
//! use stubborn_io::tokio::{UnderlyingIo, StubbornIo};
//! use tokio::fs::File;
//! use std::pin::Pin;
//! use std::future::Future;
//! use std::error::Error;
//! use std::path::PathBuf;
//!
//! impl UnderlyingIo<PathBuf> for File  {
//!    // Implementing the creation function that will be used to establish an io connection.
//!    fn create(path: PathBuf) -> Pin<Box<dyn Future<Output = Result<Self, Box<dyn Error>>>>> {
//!        Box::pin(async move {
//!            Ok(File::create(path).await?)
//!        })
//!    }
//! }
//!  
//! type HomemadeStubbornFile = StubbornIo<File, PathBuf>;
//! let path = PathBuf::from("./foo/bar.txt");
//!
//! let connect_future = HomemadeStubbornFile::connect(&path);
//! // ... application logic here
//! ```

pub mod config;

pub mod tokio;

#[doc(inline)]
pub use self::config::ReconnectOptions;
#[doc(inline)]
pub use self::tokio::StubbornTcpStream;
