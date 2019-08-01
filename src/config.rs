use std::time::Duration;

pub struct ReconnectOptions {
    pub(crate) retries_to_attempt_fn: Box<dyn Fn() -> Box<dyn Iterator<Item = Duration>>>,
    pub(crate) exit_if_first_connect_fails: bool,
}

impl ReconnectOptions {
    pub fn new() -> Self {
        ReconnectOptions {
            retries_to_attempt_fn: Box::new(get_standard_reconnect_strategy),
            exit_if_first_connect_fails: false,
        }
    }
}

fn get_standard_reconnect_strategy() -> Box<dyn Iterator<Item = Duration>> {
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
