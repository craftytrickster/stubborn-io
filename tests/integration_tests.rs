use core::time;
use std::time::Duration;

use stubborn_io::StubbornTcpStream;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::oneshot,
};

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
async fn reconnection_successful() {
    let std_listener = std::net::TcpListener::bind("0.0.0.0:0").unwrap();
    let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();
    let addr = listener.local_addr().unwrap();
    let sender_handle =
        tokio::spawn(async move { send_three_times(listener, Duration::from_millis(10)).await });

    let mut connection = StubbornTcpStream::connect(addr).await.unwrap();

    let receive_handle = tokio::spawn(async move {
        assert_eq!(connection.read_i32().await.unwrap(), 0);
        assert_eq!(connection.read_i32().await.unwrap(), 1);
        assert_eq!(connection.read_i32().await.unwrap(), 2);

        assert_eq!(connection.read_i32().await.unwrap(), 0);
        assert_eq!(connection.read_i32().await.unwrap(), 1);
        assert_eq!(connection.read_i32().await.unwrap(), 2);
    });

    sender_handle.await.unwrap();

    // Reuse previous port
    let listener = loop {
        let Ok(listener) = tokio::net::TcpListener::bind(addr).await else {
            continue;
        };
        break listener;
    };
    let sender_handle =
        tokio::spawn(async move { send_three_times(listener, Duration::from_millis(10)).await });

    receive_handle.await.unwrap();
    sender_handle.await.unwrap();
}

async fn send_three_times(listener: tokio::net::TcpListener, interval: time::Duration) {
    let (mut socket, _) = listener.accept().await.unwrap();
    for i in 0..3 {
        socket.write_i32(i).await.unwrap();
        tokio::time::sleep(interval).await;
    }
    return;
}
