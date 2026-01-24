use futures::stream::StreamExt;
use std::io::{self, ErrorKind};
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::Duration;
use stubborn_io::ReconnectOptions;
use tokio_util::codec::{Framed, LinesCodec};
use tokio_util::sync::CancellationToken;

mod common;

use common::{DummyCtor, StubbornDummy};

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
