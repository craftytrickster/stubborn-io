use super::io::{StubbornIo, UnderlyingIo};
use std::error::Error;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use tokio::net::TcpStream;

impl UnderlyingIo<SocketAddr> for TcpStream {
    fn create(addr: SocketAddr) -> Pin<Box<dyn Future<Output = Result<Self, Box<dyn Error>>>>> {
        let result = async move {
            match TcpStream::connect(&addr).await {
                Ok(tcp) => Ok(tcp),
                Err(e) => Err(Box::new(e).into()),
            }
        };

        Box::pin(result)
    }
}

pub type StubbornTcpStream = StubbornIo<TcpStream, SocketAddr>;
