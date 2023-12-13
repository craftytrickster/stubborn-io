use std::time::Duration;

use stubborn_io::StubbornTcpStream;
use tokio::{io::AsyncWriteExt, sync::oneshot};

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
