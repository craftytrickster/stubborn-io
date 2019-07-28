#![feature(async_await)]

use tokio::net::TcpStream;
use std::task::{Context, Poll};
use std::net::SocketAddr;
use std::pin::Pin;
use std::io;
use tokio::io::{AsyncRead, AsyncWrite, ErrorKind};
use bytes::{Buf, BufMut};
use std::ops::{Deref, DerefMut, Add};
use std::future::Future;
use std::time::{Duration, Instant};
use tokio::timer::Delay;



struct ReconnectOptions {
    retries_to_attempt_fn: Box<dyn Fn() -> Box<dyn Iterator<Item=Duration>>>
}

struct AttemptsTracker {
    attempt_num: usize,
    retries_remaining: Box<dyn Iterator<Item=Duration>>
}

struct ReconnectStatus {
    attempts_tracker: AttemptsTracker,
    reconnect_attempt: Pin<Box<dyn Future<Output=io::Result<TcpStream>>>>
}


fn get_standard_reconnect_strategy() -> Box<dyn Iterator<Item=Duration>> {
    let initial_attempts = vec![
        Duration::from_secs(5),
        Duration::from_secs(10),
        Duration::from_secs(20),
        Duration::from_secs(30),
        Duration::from_secs(40),
        Duration::from_secs(50),
        Duration::from_secs(60),
        Duration::from_secs(60 * 2),
        Duration::from_secs(60 * 5),
        Duration::from_secs(60 * 10),
        Duration::from_secs(60 * 20),
    ];

    let repeat = std::iter::repeat(Duration::from_secs(60 * 30));

    let forever_iterator = initial_attempts.into_iter().chain(repeat.into_iter());
    Box::new(forever_iterator)
}

impl ReconnectOptions {
    pub fn new() -> Self {
        
        ReconnectOptions {
            retries_to_attempt_fn: Box::new(get_standard_reconnect_strategy)
        }
    }
}

impl ReconnectStatus {
    pub fn new(options: &ReconnectOptions) -> Self {
        ReconnectStatus {
            attempts_tracker: AttemptsTracker {
                attempt_num: 0,
                retries_remaining: (options.retries_to_attempt_fn)()
            },
            reconnect_attempt: Box::pin(futures::future::err(
                // This is to avoid making the reconnect_attempt an Option<Future>
                io::Error::new(ErrorKind::NotConnected, "Start in disconnected state")
            ))
        }
    }
}


pub struct StubbornTcpStream {
    status: Status,
    addr: SocketAddr,
    stream: TcpStream,
    options: ReconnectOptions
}

enum Status {
    Connected,
    Disconnected(ReconnectStatus) 
    // prob need finished status here
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

impl Deref for StubbornTcpStream {
    type Target = TcpStream;

    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

impl DerefMut for StubbornTcpStream {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stream
    }
}


// these should be part of the trait, alongside the is error fatal
// that way it can be specialized for the read 0 case for the tcp one
fn is_read_disconnect_detected(poll_result: &Poll<io::Result<usize>>) -> bool {
    match poll_result {
        Poll::Ready(Ok(size)) if *size == 0 => true, // perhaps this is only true in tcp
        Poll::Ready(Err(err)) => is_error_fatal(err),
        _ => false
    }
}

fn is_write_disconnect_detected<T>(poll_result: &Poll<io::Result<T>>) -> bool {
    match poll_result {
        Poll::Ready(Err(err)) => is_error_fatal(err),
        _ => false
    }
}


impl StubbornTcpStream {
    pub async fn connect(addr: &SocketAddr) -> io::Result<Self> {
        let tcp = TcpStream::connect(addr).await?;
        let status = Status::Connected;
        let options = ReconnectOptions::new();

        Ok(StubbornTcpStream { status, addr: *addr, stream: tcp, options })
    }
    
    fn on_disconnect(mut self: Pin<&mut Self>, cx: &mut Context) {
        match &mut self.status {
            // initial disconnect
            Status::Connected => {
                println!("Disconnect occured");
                self.status = Status::Disconnected(ReconnectStatus::new(&self.options));
            },
            Status::Disconnected(_) => {}
        };
        
        let addr = self.addr;

        // this is ensured to be true now
        if let Status::Disconnected(reconnect_status) = &mut self.status {
            let next_duration = reconnect_status.attempts_tracker.retries_remaining.next().expect("You idiots!!!");

            let future_instant = Delay::new(Instant::now().add(next_duration));
            let reconnect_attempt = async move {
                future_instant.await;
                TcpStream::connect(&addr).await
            };

            reconnect_status.reconnect_attempt = Box::pin(reconnect_attempt);
            reconnect_status.attempts_tracker.attempt_num += 1;

            println!("Will attempt number: {}", reconnect_status.attempts_tracker.attempt_num);

            cx.waker().wake_by_ref();
        }
    }
    
    fn poll_disconnect(mut self: Pin<&mut Self>, cx: &mut Context) -> bool {
        let attempt = match &mut self.status {
            Status::Connected => panic!("Serious error ocurred!"),
            Status::Disconnected(ref mut status) => Pin::new(&mut status.reconnect_attempt)
        };         
        
        cx.waker().wake_by_ref();
        
        match attempt.poll(cx) {
            Poll::Ready(Ok(stream)) => {
                println!("Connection re-established");
                self.status = Status::Connected;
                self.stream = stream;
                true
            },
            Poll::Ready(Err(err)) => {
                self.on_disconnect(cx);
                false
            },
            Poll::Pending => false
        }
    }
}

// ===== impl Read / Write =====

impl AsyncRead for StubbornTcpStream {
    unsafe fn prepare_uninitialized_buffer(&self, buf: &mut [u8]) -> bool {
        match &self.status {
            Status::Connected => {
                self.stream.prepare_uninitialized_buffer(buf)
            },
            Status::Disconnected(_) => {
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
            Status::Connected => {
                let poll = AsyncRead::poll_read(Pin::new(&mut self.stream), cx, buf);
                
                if is_read_disconnect_detected(&poll) {
                    self.on_disconnect(cx);
                    Poll::Pending
                } else {
                    poll
                }
            },
            Status::Disconnected(_) => {
                self.poll_disconnect(cx);
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
            Status::Connected => {
                let poll = AsyncRead::poll_read_buf(Pin::new(&mut self.stream), cx, buf);

                if is_read_disconnect_detected(&poll) {
                    self.on_disconnect(cx);
                    Poll::Pending
                } else {
                    poll
                }
            },
            Status::Disconnected(_) => {
                self.poll_disconnect(cx);
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
            Status::Connected => {
                let poll = AsyncWrite::poll_write(Pin::new(&mut self.stream), cx, buf);
                
                if is_write_disconnect_detected(&poll) {
                    self.on_disconnect(cx);
                    Poll::Pending
                } else {
                    poll
                }
            },
            Status::Disconnected(_) => {
                self.poll_disconnect(cx);
                Poll::Pending
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.status {
            Status::Connected => {
                let poll = AsyncWrite::poll_flush(Pin::new(&mut self.stream), cx);

                if is_write_disconnect_detected(&poll) {
                    self.on_disconnect(cx);
                    Poll::Pending
                } else {
                    poll
                }
            },
            Status::Disconnected(_) => {
                self.poll_disconnect(cx);
                Poll::Pending
            }
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.status {
            Status::Connected => {
                let poll = AsyncWrite::poll_shutdown(Pin::new(&mut self.stream), cx);
                if let Poll::Ready(_) = poll {
                    // if completed, we are disconnected whether error or not
                    self.on_disconnect(cx);
                }

                poll
            },
            Status::Disconnected(_) => {
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
            Status::Connected => {
                let poll = AsyncWrite::poll_write_buf(Pin::new(&mut self.stream), cx, buf);

                if is_write_disconnect_detected(&poll) {
                    self.on_disconnect(cx);
                    Poll::Pending
                } else {
                    poll
                }
            },
            Status::Disconnected(_) => {
                self.poll_disconnect(cx);
                Poll::Pending
            }
        }
    }
}
