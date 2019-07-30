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
use std::error::Error;
use std::marker::PhantomData;
use futures::future::Ready;
use std::borrow::Borrow;

trait UnderlyingIo<C> : Sized + Unpin where C: Clone + Unpin {
    fn create(ctor_arg: C) -> Pin<Box<dyn Future<Output=Result<Self, Box<dyn Error>>>>>;
}

impl UnderlyingIo<SocketAddr> for TcpStream {
    fn create(addr: SocketAddr) -> Pin<Box<dyn Future<Output=Result<Self, Box<dyn Error>>>>> {
        let result = async move {
            match TcpStream::connect(&addr).await {
                Ok(tcp) => Ok(tcp),
                Err(e) => Err(Box::new(e).into())
            }
        };
        
        Box::pin(result)
    }
}

pub type StubbornTcpStream = StubbornIo<TcpStream, SocketAddr>;


struct ReconnectOptions {
    retries_to_attempt_fn: Box<dyn Fn() -> Box<dyn Iterator<Item=Duration>>>,
    exit_if_first_connect_fails: bool
}

struct AttemptsTracker {
    attempt_num: usize,
    retries_remaining: Box<dyn Iterator<Item=Duration>>
}

struct ReconnectStatus<T, C> {
    attempts_tracker: AttemptsTracker,
    reconnect_attempt: Pin<Box<dyn Future<Output=Result<T, Box<dyn std::error::Error>>>>>,
    _phantom_data: PhantomData<C>
}


fn get_standard_reconnect_strategy() -> Box<dyn Iterator<Item=Duration>> {
    let initial_attempts = vec![
        Duration::from_secs(3),
        Duration::from_secs(3),
        Duration::from_secs(3),
    ];

//    let initial_attempts = vec![
//        Duration::from_secs(5),
//        Duration::from_secs(10),
//        Duration::from_secs(20),
//        Duration::from_secs(30),
//        Duration::from_secs(40),
//        Duration::from_secs(50),
//        Duration::from_secs(60),
//        Duration::from_secs(60 * 2),
//        Duration::from_secs(60 * 5),
//        Duration::from_secs(60 * 10),
//        Duration::from_secs(60 * 20),
//    ];
//
//    let repeat = std::iter::repeat(Duration::from_secs(60 * 30));
//
//    let forever_iterator = initial_attempts.into_iter().chain(repeat.into_iter());

    
    
    let forever_iterator = initial_attempts.into_iter();
    Box::new(forever_iterator)
}

impl ReconnectOptions {
    pub fn new() -> Self {
        
        ReconnectOptions {
            retries_to_attempt_fn: Box::new(get_standard_reconnect_strategy),
            exit_if_first_connect_fails: false
        }
    }
}

impl<T, C> ReconnectStatus<T, C> where T: UnderlyingIo<C>, C: Clone + Unpin + 'static {
    pub fn new(options: &ReconnectOptions) -> Self {
        ReconnectStatus {
            attempts_tracker: AttemptsTracker {
                attempt_num: 0,
                retries_remaining: (options.retries_to_attempt_fn)()
            },
            reconnect_attempt: Box::pin(async { unreachable!("Not going to happen") }), // rethink this
            _phantom_data: PhantomData
        }
    }
}


pub struct StubbornIo<T, C> {
    status: Status<T, C>,
    stream: T,
    options: ReconnectOptions,
    ctor_arg: C
}

enum Status<T, C> {
    Connected,
    Disconnected(ReconnectStatus<T, C>),
    FailedAndExhausted // the way one feels after programming in dynamically typed languages
}

fn exhausted_err<T>() -> Poll<io::Result<T>> {
    let io_err = io::Error::new(ErrorKind::NotConnected, "Disconnected. Connection attempts have been exhausted.");
    Poll::Ready(Err(io_err))
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

impl<T, C> Deref for StubbornIo<T, C> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

impl<T, C> DerefMut for StubbornIo<T, C> {
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


impl<T, C> StubbornIo<T, C> where T: UnderlyingIo<C>, C: Clone + Unpin + 'static {
    pub async fn connect(ctor_arg: impl Borrow<C>) -> Result<Self, Box<dyn Error>> {
        let options = ReconnectOptions::new();
        let ctor_arg = ctor_arg.borrow().clone();

        let tcp = match T::create(ctor_arg.clone()).await {
            Ok(tcp) => {
                println!("Initial connection succeeded.");
                tcp
            },
            Err(e) => {
                println!("Initial connection failed due to: {:?}.", e);
                
                if options.exit_if_first_connect_fails {
                    println!("Bailing after initial connection failure.");
                    return Err(e);
                }
                
                let mut result = Err(e);
                
                for (i, duration) in (options.retries_to_attempt_fn)().enumerate() {
                    let reconnect_num = i + 1;
                    
                    println!(
                        "Will re-perform initial connect attempt #{} in {:?}.",
                        reconnect_num,
                        duration
                    );
                    
                    Delay::new(Instant::now().add(duration)).await;

                    println!("Attempting reconnect #{} now.", reconnect_num);

                    match T::create(ctor_arg.clone()).await {
                        Ok(tcp) => {
                            result = Ok(tcp);
                            println!("Initial connection successfully established.");
                            break;
                        },
                        Err(e) => { result = Err(e); }
                    }
                }
                
                match result {
                    Ok(tcp) => tcp,
                    Err(e) => return Err(e)
                }
            }
        };
        
        Ok(StubbornIo { status: Status::Connected, ctor_arg, stream: tcp, options })
    }
    
    fn on_disconnect(mut self: Pin<&mut Self>, cx: &mut Context) {
        match &mut self.status {
            // initial disconnect
            Status::Connected => {
                println!("Disconnect occurred");
                self.status = Status::Disconnected(ReconnectStatus::new(&self.options));
            },
            Status::Disconnected(_) => {},
            Status::FailedAndExhausted => unreachable!("on_disconnect will not occur for already exhausted state.")
        };
        
        let addr = self.ctor_arg.clone();

        // this is ensured to be true now
        if let Status::Disconnected(reconnect_status) = &mut self.status {
            
            let next_duration = match reconnect_status.attempts_tracker.retries_remaining.next() {
                Some(duration) => duration,
                None => {
                    println!("No more re-connect retries remaining. Giving up.");
                    self.status = Status::FailedAndExhausted;
                    return;
                }
            };

            let future_instant = Delay::new(Instant::now().add(next_duration));

            reconnect_status.attempts_tracker.attempt_num += 1;
            let cur_num = reconnect_status.attempts_tracker.attempt_num;

            let reconnect_attempt = async move {
                future_instant.await;
                println!("Attempting reconnect #{} now.", cur_num);
                T::create(addr).await
            };

            reconnect_status.reconnect_attempt = Box::pin(reconnect_attempt);

            println!(
                "Will perform reconnect attempt #{} in {:?}.",
                reconnect_status.attempts_tracker.attempt_num,
                next_duration
            );

            cx.waker().wake_by_ref();
        }
    }
    
    fn poll_disconnect(mut self: Pin<&mut Self>, cx: &mut Context){
        let (attempt, attempt_num) = match &mut self.status {
            Status::Connected => panic!("Serious error ocurred!"),
            Status::Disconnected(ref mut status) => (Pin::new(&mut status.reconnect_attempt), status.attempts_tracker.attempt_num),
            Status::FailedAndExhausted => unreachable!()
        };

        cx.waker().wake_by_ref();
        
        match attempt.poll(cx) {
            Poll::Ready(Ok(stream)) => {
                println!("Connection re-established");
                self.status = Status::Connected;
                self.stream = stream;
            },
            Poll::Ready(Err(err)) => {
                println!("Connection attempt #{} failed: {:?}", attempt_num, err);
                self.on_disconnect(cx);
            },
            Poll::Pending => {}
        }
    }
}

// ===== impl Read / Write =====

impl<T, C> AsyncRead for StubbornIo<T, C> where T: UnderlyingIo<C> + AsyncRead, C: Clone + Unpin + 'static {
    unsafe fn prepare_uninitialized_buffer(&self, buf: &mut [u8]) -> bool {
        match &self.status {
            Status::Connected => {
                self.stream.prepare_uninitialized_buffer(buf)
            },
            Status::Disconnected(_) => {
                false
            },
            Status::FailedAndExhausted => {
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
            },
            Status::FailedAndExhausted => exhausted_err()
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
            },
            Status::FailedAndExhausted => exhausted_err()
        }
    }
}

impl<T, C> AsyncWrite for StubbornIo<T, C> where T: UnderlyingIo<C> + AsyncWrite, C: Clone + Unpin + 'static {
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
            },
            Status::FailedAndExhausted => exhausted_err()
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
            },
            Status::FailedAndExhausted => exhausted_err()
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
            },
            Status::FailedAndExhausted => exhausted_err()
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
            },
            Status::FailedAndExhausted => exhausted_err()
        }
    }
}
