use std::time::{Duration, Instant};

use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tokio::io::AsyncReadExt;
use tokio::{io::AsyncWriteExt, sync::oneshot};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn back_to_back_shutdown_attempts() {
    let (port_tx, port_rx) = oneshot::channel();
    tokio::spawn(async move {
        let mut streams = Vec::new();
        let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        port_tx.send(addr).unwrap();
        loop {
            let (stream, _addr) = listener.accept().await.unwrap();
            streams.push(stream);
        }
    });
    let addr = port_rx.await.unwrap();
    let mut connection = StubbornTcpStream::connect(addr).await.unwrap();

    connection.shutdown().await.unwrap();
    let elapsed = tokio::time::timeout(Duration::from_secs(5), connection.shutdown()).await;

    let result = elapsed.unwrap();
    let error = result.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::NotConnected);
}

#[tokio::test]
async fn cancellation_should_halt_broken_io() {
    let (port_tx, port_rx) = oneshot::channel();
    let (close_listener_tx, close_listener_rx) = oneshot::channel();

    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        port_tx.send(addr).unwrap();

        let (stream, _addr) = listener.accept().await.unwrap();
        let _ = close_listener_rx.await;
        drop(stream);
    });

    let addr = port_rx.await.unwrap();
    let cancellation_token = CancellationToken::new();
    let config = ReconnectOptions::new()
        .with_cancel_token(cancellation_token.clone())
        .with_retries_generator(|| {
            vec![
                Duration::from_secs(45),
                Duration::from_secs(45),
                Duration::from_secs(45),
            ]
        });

    let mut connection = StubbornTcpStream::connect_with_options(addr, config)
        .await
        .unwrap();

    // give time for thing to connect
    tokio::time::sleep(Duration::from_secs(1)).await;

    // abort the listening end so the tokio tries to reconnect
    let _ = close_listener_tx.send(());

    let now = Instant::now();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;
        cancellation_token.cancel();
    });

    // trying to read from the broken connection will repeat the stubborn connection,
    // but, after the 3 second mark when we cancel the token above, it should error out
    assert!(connection.read_f32().await.is_err());
    let later = Instant::now();

    let diff = later - now;
    // the diff should roughly 3 seconds + a little overhead (ex: 3.02 seconds)
    assert!(diff > Duration::from_secs(3));
    // giving 20 second margin in case there is an insane, pathological case in which this operates very slowly
    assert!(diff < Duration::from_secs(20));
}
