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
/// use tokio::io::AsyncWriteExt;
///
/// let addr = "localhost:8080";
///
/// async {
///     let mut tcp_stream = StubbornTcpStream::connect(addr).await.unwrap();
///     tcp_stream.write_all(b"hello world!").await.unwrap();
/// };
/// ```
pub type StubbornTcpStream<A> = StubbornIo<TcpStream, A>;
