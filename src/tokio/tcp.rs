use super::io::{StubbornIo, UnderlyingIo};
use std::future::Future;
use std::io;
use std::pin::Pin;
use tokio::net::{TcpStream, ToSocketAddrs};

impl<A> UnderlyingIo<A> for TcpStream
where
    A: ToSocketAddrs + Sync + Send + Clone + Unpin + 'static,
{
    fn establish(addr: A) -> Pin<Box<dyn Future<Output = io::Result<Self>> + Send>> {
        Box::pin(TcpStream::connect(addr))
    }
}

/// A drop in replacement for tokio's [TcpStream](tokio::net::TcpStream), with the
/// distinction that it will automatically attempt to reconnect in the face of connectivity failures.
///
/// ```
/// use stubborn_io::StubbornTcpStream;
///
/// let addr = "localhost:8080";
/// async {
///     let tcp_stream = StubbornTcpStream::connect(addr).await.unwrap();
///     let regular_tokio_tcp_function_result = tcp_stream.peer_addr();
/// };
/// ```
pub type StubbornTcpStream<A> = StubbornIo<TcpStream, A>;
