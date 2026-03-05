use std::future::Future;
use std::io;
use std::io::ErrorKind;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use stubborn_io::tokio::{StubbornIo, UnderlyingIo};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub type PollReadResults = Arc<Mutex<Vec<(Poll<io::Result<()>>, Vec<u8>)>>>;

#[derive(Default)]
pub struct DummyIo {
    pub poll_read_results: PollReadResults,
}

#[derive(Default, Clone)]
pub struct DummyCtor {
    pub connect_outcomes: ConnectOutcomes,
    pub poll_read_results: PollReadResults,
}

pub type ConnectOutcomes = Arc<Mutex<Vec<bool>>>;

impl UnderlyingIo<DummyCtor> for DummyIo {
    fn establish(ctor: DummyCtor) -> Pin<Box<dyn Future<Output = io::Result<Self>> + Send>> {
        let mut connect_attempt_outcome_results = ctor.connect_outcomes.lock().unwrap();

        let should_succeed = connect_attempt_outcome_results.remove(0);
        if should_succeed {
            let dummy_io = DummyIo {
                poll_read_results: ctor.poll_read_results.clone(),
            };

            Box::pin(async { Ok(dummy_io) })
        } else {
            Box::pin(async { Err(io::Error::new(ErrorKind::NotConnected, "So unfortunate")) })
        }
    }
}

pub type StubbornDummy = StubbornIo<DummyIo, DummyCtor>;

impl AsyncWrite for DummyIo {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        unreachable!();
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        unreachable!();
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for DummyIo {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let cloned = self.poll_read_results.clone();
        let mut poll_read_results = cloned.lock().unwrap();

        let (result, bytes) = poll_read_results.remove(0);

        if let Poll::Ready(Err(e)) = result {
            if e.kind() == io::ErrorKind::WouldBlock {
                cx.waker().wake_by_ref();
                Poll::Pending
            } else {
                Poll::Ready(Err(e))
            }
        } else {
            buf.put_slice(&bytes);
            result
        }
    }
}
