//! Provides functionality related to asynchronous IO, including concrete
//! ready to use structs such as [StubbornTcpStream] as well as
//! the [UnderlyingIO trait](UnderlyingIo) and [StubbornIO struct](StubbornIo)
//! needed to create custom stubborn io types yourself.

mod io;
mod tcp;

pub use self::io::{StubbornIo, UnderlyingIo};

pub use self::tcp::StubbornTcpStream;
