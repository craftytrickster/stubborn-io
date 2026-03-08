use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use stubborn_io::ReconnectOptions;
use tokio::io::{AsyncRead, AsyncWrite};

mod common;
use common::{DummyCtor, StubbornDummy};

#[cfg(test)]
pub mod instantiating {
    use super::*;

    #[tokio::test]
    async fn should_be_connected_if_initial_connect_succeeds() {
        let connect_outcomes = Arc::new(Mutex::new(vec![true]));

        let ctor = DummyCtor {
            connect_outcomes,
            ..DummyCtor::default()
        };

        let dummy = StubbornDummy::connect(ctor).await;

        assert!(dummy.is_ok());
    }

    #[tokio::test]
    async fn should_be_disconnected_if_initial_connect_fails_with_fail_on_first_enabled() {
        let connect_outcomes = Arc::new(Mutex::new(vec![false, true]));
        let ctor = DummyCtor {
            connect_outcomes,
            ..DummyCtor::default()
        };

        let dummy = StubbornDummy::connect(ctor).await;

        assert!(dummy.is_err());
    }

    #[tokio::test]
    async fn should_be_disconnected_if_all_initial_connects_fail() {
        let connect_outcomes = Arc::new(Mutex::new(vec![false, false]));
        let ctor = DummyCtor {
            connect_outcomes,
            ..DummyCtor::default()
        };

        let disconnect_counter = Arc::new(AtomicU8::new(0));
        let disconnect_clone = disconnect_counter.clone();

        let options = ReconnectOptions::new()
            .with_retries_generator(|| vec![Duration::from_millis(100)])
            .with_exit_if_first_connect_fails(false)
            .with_on_connect_fail_callback(move || {
                disconnect_clone.fetch_add(1, Ordering::Relaxed);
            });

        let dummy = StubbornDummy::connect_with_options(ctor, options).await;

        assert_eq!(disconnect_counter.load(Ordering::Relaxed), 2);
        assert!(dummy.is_err());
    }

    #[tokio::test]
    async fn should_be_connected_if_initial_connect_fails_but_then_other_succeeds() {
        let connect_outcomes = Arc::new(Mutex::new(vec![false, true]));
        let ctor = DummyCtor {
            connect_outcomes,
            ..DummyCtor::default()
        };

        let disconnect_counter = Arc::new(AtomicU8::new(0));
        let disconnect_clone = disconnect_counter.clone();

        let options = ReconnectOptions::new()
            .with_exit_if_first_connect_fails(false)
            .with_retries_generator(|| vec![Duration::from_millis(100)])
            .with_on_connect_fail_callback(move || {
                disconnect_clone.fetch_add(1, Ordering::Relaxed);
            });

        let dummy = StubbornDummy::connect_with_options(ctor, options).await;

        assert_eq!(disconnect_counter.load(Ordering::Relaxed), 1);
        assert!(dummy.is_ok());
    }
}

#[cfg(test)]
mod already_connected {
    use super::*;
    use futures::stream::StreamExt;
    use std::io;
    use std::task::Poll;

    use tokio_util::codec::{Framed, LinesCodec};

    #[tokio::test]
    async fn should_ignore_non_fatal_errors_and_continue_as_connected() {
        let connect_outcomes = Arc::new(Mutex::new(vec![true]));

        let poll_read_results = Arc::new(Mutex::new(vec![
            (
                Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "good old fashioned async io msg",
                ))),
                vec![],
            ),
            (Poll::Ready(Ok(())), b"yother".to_vec()),
            (Poll::Ready(Ok(())), b"e\n".to_vec()),
        ]));

        let ctor = DummyCtor {
            connect_outcomes,
            poll_read_results,
        };

        let dummy = StubbornDummy::connect(ctor).await.unwrap();

        let mut framed = Framed::new(dummy, LinesCodec::new());

        let msg = framed.next().await.unwrap().unwrap();

        assert_eq!(msg, String::from("yothere"));
    }

    #[tokio::test]
    async fn should_be_able_to_recover_after_disconnect() {
        let connect_outcomes = Arc::new(Mutex::new(vec![true, false, true]));

        let poll_read_results = Arc::new(Mutex::new(vec![
            (
                Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "fatal",
                ))),
                vec![],
            ),
            (Poll::Ready(Ok(())), b"e\n".to_vec()),
        ]));

        let ctor = DummyCtor {
            connect_outcomes,
            poll_read_results: poll_read_results.clone(),
        };

        let disconnect_counter = Arc::new(AtomicU8::new(0));
        let disconnect_clone = disconnect_counter.clone();

        let options = ReconnectOptions::new()
            .with_on_disconnect_callback(move || {
                disconnect_clone.fetch_add(1, Ordering::Relaxed);
            })
            .with_retries_generator(|| {
                vec![
                    Duration::from_millis(100),
                    Duration::from_millis(100),
                    Duration::from_millis(100),
                ]
            });

        let dummy = StubbornDummy::connect_with_options(ctor, options)
            .await
            .unwrap();

        let mut framed = Framed::new(dummy, LinesCodec::new());

        let msg = framed.next().await;

        assert_eq!(msg.unwrap().unwrap(), String::from("e"));
        assert_eq!(disconnect_counter.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn should_give_up_when_all_attempts_exhausted() {
        let connect_outcomes = Arc::new(Mutex::new(vec![true, false, false, false]));

        let poll_read_results = Arc::new(Mutex::new(vec![
            (
                Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "fatal",
                ))),
                vec![],
            ),
            (Poll::Ready(Ok(())), b"e\n".to_vec()),
        ]));

        let ctor = DummyCtor {
            connect_outcomes,
            poll_read_results: poll_read_results.clone(),
        };

        let options = ReconnectOptions::new().with_retries_generator(|| {
            vec![
                Duration::from_millis(100),
                Duration::from_millis(100),
                Duration::from_millis(100),
            ]
        });

        let dummy = StubbornDummy::connect_with_options(ctor, options)
            .await
            .unwrap();

        let mut framed = Framed::new(dummy, LinesCodec::new());

        let msg = framed.next().await;

        assert!(msg.unwrap().is_err());
    }
}

#[tokio::test]
async fn test_that_works_with_sync() {
    fn make_framed<T>(_stream: T)
    where
        T: AsyncRead + AsyncWrite + Send + Sync + 'static,
    {
        let _ = _stream;
    }

    let options = ReconnectOptions::new();
    let connect_outcomes = Arc::new(Mutex::new(vec![true]));
    let ctor = DummyCtor {
        connect_outcomes,
        ..DummyCtor::default()
    };
    let dummy = StubbornDummy::connect_with_options(ctor, options)
        .await
        .unwrap();

    make_framed(dummy);
}
