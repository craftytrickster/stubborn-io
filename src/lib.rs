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
                println!("C:prepare_uninitialized_buffer");
                tcp.prepare_uninitialized_buffer(buf)
            },
            Status::Disconnected => {
                println!("D:prepare_uninitialized_buffer");
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
                println!("C:poll_read");
                let poll_read = AsyncRead::poll_read(Pin::new(tcp), cx, buf);
                
                match poll_read {
                    Poll::Ready(Ok(size)) if size == 0 => {
                        self.on_disconnect();
                        Poll::Pending
                    },
                    Poll::Ready(Err(ref e)) => {
                        if is_error_fatal(e) {
                            self.on_disconnect();
                            Poll::Pending
                        } else {
                            poll_read
                        }
                    }
                    _ => poll_read
                }
            },
            Status::Disconnected => {
                println!("D:poll_read");
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
                println!("C:poll_read_buf");
                let poll_read_buf = AsyncRead::poll_read_buf(Pin::new(tcp), cx, buf);

                match poll_read_buf {
                    Poll::Ready(Ok(size)) if size == 0 => {
                        self.on_disconnect();
                        Poll::Pending
                    }
                    Poll::Ready(Err(ref e)) => {
                        if is_error_fatal(e) {
                            self.on_disconnect();
                            Poll::Pending
                        } else {
                            poll_read_buf
                        }
                    }
                    _ => poll_read_buf
                }
            },
            Status::Disconnected => {
                println!("D:poll_read_buf");
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
                println!("C:poll_write");
                let write_result = AsyncWrite::poll_write(Pin::new(tcp), cx, buf);
                println!("WRR: {:?}", write_result);
                match write_result {
                    Poll::Ready(Err(ref e)) if is_error_fatal(e) => {
                        self.on_disconnect();
                        Poll::Pending
                    },
                    _ => write_result
                }
            },
            Status::Disconnected => {
                println!("D:poll_write");
                Poll::Pending
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.status {
            Status::Connected(ref mut tcp) => {
                println!("C:poll_flush");
                let flush = AsyncWrite::poll_flush(Pin::new(tcp), cx);
                match flush {
                    Poll::Ready(Err(ref e)) if is_error_fatal(e) => {
                        self.on_disconnect();
                        Poll::Pending
                    },
                    _ => flush
                }
            },
            Status::Disconnected => {
                println!("D:poll_flush");
                Poll::Pending
            }
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.status {
            Status::Connected(ref mut tcp) => {
                println!("C:poll_shutdown");
                match AsyncWrite::poll_shutdown(Pin::new(tcp), cx) {
                    Poll::Ready(_) => { // if completed, we are disconnected whether error or not
                        self.on_disconnect();
                    },
                    _ => {}
                };

                Poll::Pending
            },
            Status::Disconnected => {
                println!("D:poll_shutdown");
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
                println!("C:poll_write_buf");
                let poll_write_buf = AsyncWrite::poll_write_buf(Pin::new(tcp), cx, buf);
                
                match poll_write_buf {
                    Poll::Ready(Err(ref e)) if is_error_fatal(e) => {
                        self.on_disconnect();
                        Poll::Pending
                    },
                    _ => poll_write_buf
                }
            },
            Status::Disconnected => {
                println!("D:poll_write_buf");
                Poll::Pending
            }
        }
    }
}
