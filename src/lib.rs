#![feature(async_await)]

pub mod config;
pub mod tokio;

pub mod prelude {
    pub use super::config::ReconnectOptions;
    pub use super::tokio::StubbornTcpStream;
}
