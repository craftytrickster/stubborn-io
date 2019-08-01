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
    
    pub fn with_retries_generator<F, I, IN>(mut self, retries_generator: F) -> Self
    where
        F: 'static + Fn() -> IN,
        I: 'static + Iterator<Item = Duration>,
        IN: IntoIterator<IntoIter = I, Item = Duration>,
    {
        self.set_retries_generator(retries_generator);
        self
    }

    pub fn set_retries_generator<F, I, IN>(&mut self, retires_generator: F)
    where
        F: 'static + Fn() -> IN,
        I: 'static + Iterator<Item = Duration>,
        IN: IntoIterator<IntoIter = I, Item = Duration>,
    {
        self.retries_to_attempt_fn = Box::new(move || Box::new(retires_generator().into_iter()));
    }

    pub fn with_exit_on_initial_fail(mut self, exit: bool) -> Self {
        self.set_exit_on_initial_fail(exit);
        self
    }

    pub fn set_exit_on_initial_fail(&mut self, exit: bool) {
        self.exit_if_first_connect_fails = exit;
    }
}

fn get_standard_reconnect_strategy() -> Box<dyn Iterator<Item = Duration>> {
    let initial_attempts = vec![
        Duration::from_secs(5),
        Duration::from_secs(10),
        Duration::from_secs(20),
        Duration::from_secs(30),
        Duration::from_secs(40),
        Duration::from_secs(50),
        Duration::from_secs(60),
        Duration::from_secs(60 * 2),
        Duration::from_secs(60 * 5),
        Duration::from_secs(60 * 10),
        Duration::from_secs(60 * 20),
    ];

    let repeat = std::iter::repeat(Duration::from_secs(60 * 30));

    let forever_iterator = initial_attempts.into_iter().chain(repeat.into_iter());

    Box::new(forever_iterator)
}
