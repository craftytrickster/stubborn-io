#![feature(async_await)]

use stubborn_stream::{StubbornTcpStream};
use tokio;
use std::net::{SocketAddr, SocketAddrV4};
use std::future::Future;
use std::pin::Pin;
use std::task::{Poll, Context};
use std::thread;
use std::io::{stdin};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::codec::{Framed, LinesCodec};
use futures::{Sink, Stream};


struct MainLoop {
    framed: Framed<StubbornTcpStream, LinesCodec>,
    rx: UnboundedReceiver<String>
}

impl MainLoop {
    fn new(stub: StubbornTcpStream, rx: UnboundedReceiver<String>) -> Self {
        let framed = Framed::new(stub, LinesCodec::new());
        
        MainLoop { framed, rx }
    }
}

impl Future for MainLoop {
    type Output = ();
    
    fn poll(mut self: Pin<& mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let _ = self.rx.poll_recv(cx).map(|item| {
            let _ = Pin::new(&mut self.framed).poll_flush(cx);
            let _ = Pin::new(&mut self.framed).start_send(item.unwrap());
            let _ = Pin::new(&mut self.framed).poll_flush(cx);

            cx.waker().wake_by_ref();
        });


        match Pin::new(&mut self.framed).poll_next(cx) {
            Poll::Ready(Some(item)) => {
                println!("FROM SERVER: {:?}", item);
                if let Err(_) = item {
                    return Poll::Ready(());
                }
                cx.waker().wake_by_ref();
            },
            Poll::Ready(None) => {
                println!("YOU ARE NOTHING");
            }
            Poll::Pending => {
//                println!("GRound control to pending tom");
            }
        };
        
        Poll::Pending
    }
}



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut tx, rx) = mpsc::unbounded_channel();
    thread::spawn(move || {
        loop {
            let mut line = String::new();
            let _ = stdin().read_line(&mut line);
            let _ = tx.try_send(line);
        }
    });



    let addr: SocketAddrV4 = "127.0.0.1:2000".parse().unwrap();
    let addr = SocketAddr::V4(addr);
    
    let connection = StubbornTcpStream::connect(&addr).await.expect("Where's the connection");
    let thing = MainLoop::new(connection, rx);

    thing.await;
    Ok(())
}