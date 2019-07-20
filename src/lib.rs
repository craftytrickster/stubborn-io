#![feature(async_await)]

use tokio::net::TcpStream;
use std::task::{Context, Poll};
use std::net::SocketAddr;
use std::pin::Pin;
use std::io;
use tokio::io::{AsyncRead, AsyncWrite};
use bytes::{Buf, BufMut};

pub struct StubbornTcpStream {
    tcp: TcpStream
}

impl StubbornTcpStream {
    pub async fn connect(addr: &SocketAddr) -> io::Result<Self> {
        let tcp = TcpStream::connect(addr).await?;
        Ok(StubbornTcpStream { tcp })
    }

    pub fn poll_peek(&mut self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        self.tcp.poll_peek(cx, buf)
    }
}

// ===== impl Read / Write =====

impl AsyncRead for StubbornTcpStream {
    unsafe fn prepare_uninitialized_buffer(&self, _: &mut [u8]) -> bool {
        false
    }

    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.tcp.poll_read(cx, buf)
    }

    fn poll_read_buf<B: BufMut>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut B,
    ) -> Poll<io::Result<usize>> {
        self.tcp.poll_read_buf(cx, buf)
    }
}

impl AsyncWrite for StubbornTcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.tcp.poll_write(cx, buf)
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // tcp flush is a no-op
        self.tcp.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.tcp.poll_shutdown(cx)
    }

    fn poll_write_buf<B: Buf>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut B,
    ) -> Poll<io::Result<usize>> {
        self.tcp.poll_write_buf(cx, buf)
    }
}