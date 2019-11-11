use super::io::{StubbornIo, UnderlyingIo};
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use tokio::net::TcpStream;

impl UnderlyingIo<SocketAddr> for TcpStream {
    fn establish(addr: SocketAddr) -> Pin<Box<dyn Future<Output = io::Result<Self>> + Send>> {
        Box::pin(TcpStream::connect(addr))
    }
}

/// A drop in replacement for tokio's [TcpStream](tokio::net::TcpStream), with the
/// distinction that it will automatically attempt to reconnect in the face of connectivity failures.
///
/// ```
/// use std::net::{IpAddr, Ipv4Addr, SocketAddr};
/// use stubborn_io::StubbornTcpStream;
///
/// let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
/// async {
///     let tcp_stream = StubbornTcpStream::connect(&addr).await.unwrap();
///     let regular_tokio_tcp_function_result = tcp_stream.peer_addr();
/// };
/// ```
pub type StubbornTcpStream = StubbornIo<TcpStream, SocketAddr>;
