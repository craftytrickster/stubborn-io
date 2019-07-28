#![feature(async_await)]

use tokio::net::TcpStream;
use std::task::{Context, Poll};
use std::net::SocketAddr;
use std::pin::Pin;
use std::io;
use tokio::io::{AsyncRead, AsyncWrite, ErrorKind};
use bytes::{Buf, BufMut};
use mio::channel::SendError::Disconnected;


pub struct StubbornTcpStream {
    status: Status,
    addr: SocketAddr,
    // maybe always keep stream ref here, and then just mem_swap it when necessary?
}

enum Status {
    Connected(TcpStream),
    Disconnected
}

// should be customizable by user
fn is_error_fatal(err: &std::io::Error) -> bool {
    use std::io::ErrorKind::*;
    
    match err.kind() {
        NotFound |
        PermissionDenied |
        ConnectionRefused |
        ConnectionReset |
        ConnectionAborted |
        NotConnected | 
        AddrInUse |
        AddrNotAvailable |
        BrokenPipe |
        AlreadyExists => true,
        _ => false
    }
}

//impl Deref for StubbornTcpStream {
//    type Target = TcpStream;
//
//    fn deref(&self) -> &Self::Target {
//        &self.tcp
//    }
//}

// these should be part of the trait, alongside the is error fatal
// that way it can be specialized for the read 0 case for the tcp one
fn is_read_disconnect_detected(poll_result: &Poll<std::io::Result<usize>>) -> bool {
    match poll_result {
        Poll::Ready(Ok(size)) if *size == 0 => true, // perhaps this is only true in tcp
        Poll::Ready(Err(err)) => is_error_fatal(err),
        _ => false
    }
}

fn is_write_disconnect_detected<T>(poll_result: &Poll<std::io::Result<T>>) -> bool {
    match poll_result {
        Poll::Ready(Err(err)) => is_error_fatal(err),
        _ => false
    }
}


impl StubbornTcpStream {
    pub async fn connect(addr: &SocketAddr) -> io::Result<Self> {
        let tcp = TcpStream::connect(addr).await?;
        let status = Status::Connected(tcp);

        Ok(StubbornTcpStream { status, addr: *addr })
    }
    
    fn on_disconnect(&mut self) {
        println!("Disconnect occured");
        self.status = Status::Disconnected; 
    }
}

// ===== impl Read / Write =====

impl AsyncRead for StubbornTcpStream {
    unsafe fn prepare_uninitialized_buffer(&self, buf: &mut [u8]) -> bool {
        match &self.status {
            Status::Connected(tcp) => {
                tcp.prepare_uninitialized_buffer(buf)
            },
            Status::Disconnected => {
                false
            }
        }
    }

    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match &mut self.status {
            Status::Connected(ref mut tcp) => {
                let poll = AsyncRead::poll_read(Pin::new(tcp), cx, buf);
                
                if is_read_disconnect_detected(&poll) {
                    self.on_disconnect();
                    Poll::Pending
                } else {
                    poll
                }
            },
            Status::Disconnected => {
                Poll::Pending
            }
        }
    }

    fn poll_read_buf<B: BufMut>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut B,
    ) -> Poll<io::Result<usize>> {
        match &mut self.status {
            Status::Connected(ref mut tcp) => {
                let poll = AsyncRead::poll_read_buf(Pin::new(tcp), cx, buf);

                if is_read_disconnect_detected(&poll) {
                    self.on_disconnect();
                    Poll::Pending
                } else {
                    poll
                }
            },
            Status::Disconnected => {
                Poll::Pending
            }
        }
    }
}

impl AsyncWrite for StubbornTcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match &mut self.status {
            Status::Connected(ref mut tcp) => {
                let poll = AsyncWrite::poll_write(Pin::new(tcp), cx, buf);
                
                if is_write_disconnect_detected(&poll) {
                    self.on_disconnect();
                    Poll::Pending
                } else {
                    poll
                }
            },
            Status::Disconnected => {
                Poll::Pending
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.status {
            Status::Connected(ref mut tcp) => {
                let poll = AsyncWrite::poll_flush(Pin::new(tcp), cx);

                if is_write_disconnect_detected(&poll) {
                    self.on_disconnect();
                    Poll::Pending
                } else {
                    poll
                }
            },
            Status::Disconnected => {
                Poll::Pending
            }
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.status {
            Status::Connected(ref mut tcp) => {
                let poll = AsyncWrite::poll_shutdown(Pin::new(tcp), cx);
                if let Poll::Ready(_) = poll {
                    // if completed, we are disconnected whether error or not
                    self.on_disconnect();
                }

                poll
            },
            Status::Disconnected => {
                Poll::Pending
            }
        }
    }

    fn poll_write_buf<B: Buf>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut B,
    ) -> Poll<io::Result<usize>> {
        match &mut self.status {
            Status::Connected(ref mut tcp) => {
                let poll = AsyncWrite::poll_write_buf(Pin::new(tcp), cx, buf);

                if is_write_disconnect_detected(&poll) {
                    self.on_disconnect();
                    Poll::Pending
                } else {
                    poll
                }
            },
            Status::Disconnected => {
                Poll::Pending
            }
        }
    }
}
