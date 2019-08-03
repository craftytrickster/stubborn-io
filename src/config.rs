//! Provides options to configure the behavior of stubborn-io items,
//! specifically related to reconnect behavior.

use std::time::Duration;

/// User specified options that control the behavior of the stubborn-io upon disconnect.
pub struct ReconnectOptions {
    /// Represents a function that generates an Iterator
    /// to schedule the wait between reconnection attempts.
    pub retries_to_attempt_fn: Box<dyn Fn() -> Box<dyn Iterator<Item = Duration>>>,

    /// If this is set to true, if the initial connect method of the stubborn-io item fails,
    /// then no further reconnects will be attempted
    pub exit_if_first_connect_fails: bool,
}

impl ReconnectOptions {
    /// By default, the stubborn-io will try to reconnect even if the first connect attempt fails.
    /// By default, the retries iterator waits longer and longer between reconnection attempts,
    /// until it eventually perpetually tries to reconnect every 30 minutes.
    pub fn new() -> Self {
        ReconnectOptions {
            retries_to_attempt_fn: Box::new(get_standard_reconnect_strategy),
            exit_if_first_connect_fails: false,
        }
    }

    /// This convenience function allows the user to provide any function that returns a value
    /// that is convertible into an iterator, such as an actual iterator or a Vec.
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use stubborn_io::ReconnectOptions;
    ///
    /// // With the below vector, the stubborn-io item will try to reconnect three times,
    /// // waiting 2 seconds between each attempt. Once all three tries are exhausted,
    /// // it will stop attempting.
    /// let options = ReconnectOptions::new().with_retries_generator(|| {
    ///     vec![
    ///         Duration::from_secs(2),
    ///         Duration::from_secs(2),
    ///         Duration::from_secs(2),
    ///     ]
    /// });
    /// ```
    pub fn with_retries_generator<F, I, IN>(mut self, retries_generator: F) -> Self
    where
        F: 'static + Fn() -> IN,
        I: 'static + Iterator<Item = Duration>,
        IN: IntoIterator<IntoIter = I, Item = Duration>,
    {
        self.retries_to_attempt_fn = Box::new(move || Box::new(retries_generator().into_iter()));
        self
    }

    pub fn with_exit_on_initial_fail(mut self, exit: bool) -> Self {
        self.exit_if_first_connect_fails = exit;
        self
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
