use futures::stream::StreamExt;
use std::future::Future;
use std::io::{self, ErrorKind};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;
use stubborn_io::tokio::{StubbornIo, UnderlyingIo};
use stubborn_io::ReconnectOptions;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_util::codec::{Framed, LinesCodec};
use tokio_util::sync::CancellationToken;

type ConnectOutcomes = Arc<Mutex<Vec<bool>>>;
type PollReadResults = Arc<Mutex<Vec<(Poll<io::Result<()>>, Vec<u8>)>>>;
type StubbornDummy = StubbornIo<DummyIo, DummyCtor>;

#[derive(Default)]
pub struct DummyIo {
    poll_read_result: PollReadResults,
}

#[derive(Default, Clone)]
pub struct DummyCtor {
    connect_outcomes: ConnectOutcomes,
    poll_read_results: PollReadResults,
}

impl UnderlyingIo<DummyCtor> for DummyIo {
    fn establish(ctor: DummyCtor) -> Pin<Box<dyn Future<Output = io::Result<Self>> + Send>> {
        let mut connect_attempt_outcome_results = ctor.connect_outcomes.lock().unwrap();
        let should_succeed = connect_attempt_outcome_results.remove(0);

        Box::pin(async move {
            if should_succeed {
                Ok(DummyIo {
                    poll_read_result: ctor.poll_read_results,
                })
            } else {
                Err(io::Error::new(ErrorKind::NotConnected, "Not connected"))
            }
        })
    }
}

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
        let mut results = self.poll_read_result.lock().unwrap();
        let (result, bytes) = results.remove(0);

        match result {
            Poll::Ready(Err(e)) if e.kind() == ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Ok(())) => {
                buf.put_slice(&bytes);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

fn dummy_ctor(outcomes: Vec<bool>) -> DummyCtor {
    DummyCtor {
        connect_outcomes: Arc::new(Mutex::new(outcomes)),
        ..Default::default()
    }
}

fn options_with_cancel(token: CancellationToken, retries: usize) -> ReconnectOptions {
    ReconnectOptions::new()
        .with_exit_if_first_connect_fails(false)
        .with_retries_generator(move || vec![Duration::from_millis(50); retries])
        .with_cancel_token(token)
}

#[tokio::test]
async fn should_work_without_cancel_token() {
    let ctor = dummy_ctor(vec![true]);
    let options = ReconnectOptions::new();

    let result = StubbornDummy::connect_with_options(ctor, options).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn should_fail_with_pre_cancelled_token() {
    let token = CancellationToken::new();
    token.cancel();

    let ctor = dummy_ctor(vec![false, false]);
    let options = options_with_cancel(token, 1);

    let result = StubbornDummy::connect_with_options(ctor, options).await;
    assert!(matches!(result, Err(e) if e.kind() == ErrorKind::Interrupted));
}

#[tokio::test]
async fn should_cancel_initial_connection_attempts() {
    let token = CancellationToken::new();
    let token_clone = token.clone();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(120)).await;
        token_clone.cancel();
    });

    let result = StubbornDummy::connect_with_options(
        dummy_ctor(vec![false, false, false, false]),
        options_with_cancel(token, 3),
    )
    .await;

    assert!(matches!(result, Err(e) if e.kind() == ErrorKind::Interrupted));
}

#[tokio::test]
async fn should_cancel_reconnection_attempts() {
    let token = CancellationToken::new();
    let token_clone = token.clone();

    let options = ReconnectOptions::new()
        .with_retries_generator(|| vec![Duration::from_millis(50); 4])
        .with_cancel_token(token);

    let mut ctor = dummy_ctor(vec![true, false, false, false, false]);
    ctor.poll_read_results = Arc::new(Mutex::new(vec![(
        Poll::Ready(Err(io::Error::new(ErrorKind::ConnectionAborted, "fatal"))),
        vec![],
    )]));

    let dummy = StubbornDummy::connect_with_options(ctor, options)
        .await
        .unwrap();
    let mut framed = Framed::new(dummy, LinesCodec::new());

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(120)).await;
        token_clone.cancel();
    });

    let result = framed.next().await;
    assert!(result.unwrap().is_err());
}
