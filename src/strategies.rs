use crate::config::DurationIterator;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::time::Duration;

pub(crate) fn get_standard_reconnect_strategy() -> DurationIterator {
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

    let forever_iterator = initial_attempts.into_iter().chain(repeat);

    Box::new(forever_iterator)
}

/// Type used for defining the exponential backoff strategy.
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use stubborn_io::{ReconnectOptions, ExpBackoffStrategy};
///
/// // With the below strategy, the stubborn-io item will try to reconnect infinitely,
/// // waiting an exponentially increasing (by 2) value with 5% random jitter. Once the
/// // wait would otherwise exceed the maxiumum of 30 seconds, it will instead wait 30
/// // seconds.
///
/// let options = ReconnectOptions::new().with_retries_generator(|| {
///     ExpBackoffStrategy::new(Duration::from_secs(1), 2, 0.05)
///         .with_max(Duration::from_secs(30))
/// });
/// ```
pub struct ExpBackoffStrategy {
    min: Duration,
    max: Option<Duration>,
    factor: u32,
    jitter: f64,
    seed: Option<u64>,
}

impl ExpBackoffStrategy {
    pub fn new(min: Duration, factor: u32, jitter: f64) -> Self {
        Self {
            min,
            max: None,
            factor,
            jitter,
            seed: None,
        }
    }

    /// Set the exponential backoff strategy's maximum wait value to the given duration.
    /// Otherwise, this value will be the maximum possible duration.
    pub fn with_max(mut self, max: Duration) -> Self {
        self.max = Some(max);
        self
    }

    /// Set the seed used to generate jitter. Otherwise, will set RNG via entropy.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }
}

impl IntoIterator for ExpBackoffStrategy {
    type Item = Duration;
    type IntoIter = ExpBackoffIter;

    fn into_iter(self) -> Self::IntoIter {
        let init = self.min.as_secs_f64();
        let rng = match self.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_entropy(),
        };

        ExpBackoffIter {
            strategy: self,
            init,
            pow: 0,
            rng,
        }
    }
}

/// Iterator class for [ExpBackoffStrategy]
pub struct ExpBackoffIter {
    strategy: ExpBackoffStrategy,
    init: f64,
    pow: u32,
    rng: StdRng,
}

impl Iterator for ExpBackoffIter {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        self.pow += 1;
        let base = self.init * self.strategy.factor.pow(self.pow) as f64;
        let jitter = base * self.strategy.jitter * (self.rng.gen::<f64>() * 2. - 1.);
        let current = Duration::from_secs_f64(base + jitter);
        match self.strategy.max {
            Some(max) => Some(max.min(current)),
            None => Some(current),
        }
    }
}

#[cfg(test)]
mod test {
    use super::ExpBackoffStrategy;
    use std::time::Duration;

    macro_rules! assert_close_to {
        ($item:expr, $value:expr) => {
            assert_close_to!($item, $value, 0.001_f64)
        };
        ($item:expr, $value:expr, $eps: expr) => {
            assert!($item >= ($value - $eps));
            assert!($item <= ($value + $eps));
        };
    }

    #[test]
    fn test_exponential_backoff_jitter_bounds() {
        let mut backoff_iter = ExpBackoffStrategy::new(Duration::from_secs(1), 2, 0.1).into_iter();
        let first = backoff_iter.next();
        assert_close_to!(first.unwrap().as_secs_f64(), 2.0_f64, 0.2_f64);
        let second = backoff_iter.next();
        assert_close_to!(second.unwrap().as_secs_f64(), 4.0_f64, 0.4_f64);
    }

    #[test]
    fn test_exponential_backoff_max_value() {
        let mut backoff_iter = ExpBackoffStrategy::new(Duration::from_secs(1), 4, 0.0)
            .with_max(Duration::from_secs(2))
            .into_iter();
        let first = backoff_iter.next();
        assert_close_to!(first.unwrap().as_secs_f64(), 2.0);
        let second = backoff_iter.next();
        assert_close_to!(second.unwrap().as_secs_f64(), 2.0);
    }
}
