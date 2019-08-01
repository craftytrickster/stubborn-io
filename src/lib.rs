#![feature(async_await)]

pub mod tokio;

pub mod prelude {
    pub use super::tokio::{ReconnectOptions, StubbornTcpStream};
}
