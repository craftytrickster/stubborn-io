#![feature(async_await)]

pub mod tokio;
pub mod config;

pub mod prelude {
    pub use super::config::ReconnectOptions;
    pub use super::tokio::StubbornTcpStream;
}
