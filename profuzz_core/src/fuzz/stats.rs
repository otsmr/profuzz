use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use std::sync::{Arc, RwLock};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
/// A structure to hold statistics related to execution and performance metrics.
pub struct Stats {
    /// The title or name of the statistics instance.
    pub title: String,

    /// The total number of executions that have been performed.
    pub total_executions: u64,

    /// The timestamp when the execution started, if applicable.
    /// This field is `None` if the execution has not started yet.
    pub started: Option<SerializableInstant>,

    /// The number of executions performed per second.
    pub executions_per_second: ExecsPerSecond,

    /// The timestamp of the last health check performed.
    /// This field is `None` if no health check has been performed yet.
    pub last_healt_check: Option<SerializableInstant>,

    /// A boolean indicating whether the execution is currently running.
    pub running: bool,

    /// The total number of cycles completed during the execution.
    pub cylcles_done: usize,

    /// The timestamp of the last new path discovered during execution.
    /// This field is `None` if no new path has been found yet.
    pub last_new_path: Option<SerializableInstant>,

    /// The timestamp of the last unique crash encountered.
    /// This field is `None` if no unique crash has been recorded.
    pub last_unique_crash: Option<SerializableInstant>,

    /// The total number of crashes that have occurred during execution.
    pub total_crashes: usize,

    /// The total number of unique responses received during execution.
    pub total_unique_responses: usize,

    /// The count of items in the corpus, representing the number of unique inputs processed.
    pub corpus_count: usize,

    /// The total number of timeouts that have occurred during execution.
    pub total_timeouts: usize,

    /// The backoff time in milliseconds, used to manage retries or delays in execution.
    pub backoff_time: u64,
}

const BUCKET_SIZE: usize = 10;

#[derive(Clone, Debug, Serialize, Deserialize)]
/// execs per second
pub struct ExecsPerSecond {
    time_started: SerializableInstant,
    counter: [usize; BUCKET_SIZE],
    last_bucket_id: usize,
}

impl Default for ExecsPerSecond {
    fn default() -> Self {
        Self {
            time_started: SerializableInstant::now(),
            counter: Default::default(),
            last_bucket_id: 0,
        }
    }
}

impl ExecsPerSecond {
    pub(crate) fn get(&self) -> usize {
        self.counter.iter().sum()
    }

    pub(crate) fn clear(&mut self) {
        self.counter = Default::default();
    }

    pub(crate) fn start(stats: Arc<RwLock<Stats>>) {
        spawn(move || {
            loop {
                if let Ok(mut stats) = stats.write() {
                    if !stats.running {
                        break;
                    }
                    stats.executions_per_second.reseter();
                }
                sleep(Duration::from_millis((1000 / BUCKET_SIZE / 2) as u64));
            }
        });
    }

    fn reseter(&mut self) {
        let bucked_id =
            self.time_started.elapsed().as_millis() as usize % 1000 / (1000 / BUCKET_SIZE);
        if self.last_bucket_id == bucked_id {
            //self.counter[bucked_id] += 1;
        } else {
            self.counter[bucked_id] = 0;
            self.last_bucket_id = bucked_id;
        }
    }

    pub(crate) fn add(&mut self) {
        let bucked_id =
            self.time_started.elapsed().as_millis() as usize % 1000 / (1000 / BUCKET_SIZE);
        self.counter[bucked_id] += 1;
    }
}

/// Shared stats type
pub type StatsType = Arc<RwLock<Stats>>;

#[derive(Debug, Clone, Copy)]
/// A wrapper arround the std Instant to implement the `Serialize` to Instant
pub struct SerializableInstant(Instant);

impl Default for SerializableInstant {
    fn default() -> Self {
        Self(Instant::now())
    }
}

impl SerializableInstant {
    /// Wrapps the given instant into the `SerializableInstant`
    fn new(instant: Instant) -> Self {
        Self(instant)
    }
    /// Creates a new instant with the current time
    #[must_use]
    pub fn now() -> Self {
        Self(Instant::now())
    }
    /// Returns the inner `Instant`
    #[must_use]
    pub fn into_inner(self) -> Instant {
        self.0
    }
}

impl Serialize for SerializableInstant {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as duration since process start
        serializer.serialize_u128(self.0.elapsed().as_nanos())
    }
}

impl<'de> Deserialize<'de> for SerializableInstant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let nanos = u128::deserialize(deserializer)?;
        #[allow(clippy::cast_possible_truncation)]
        if let Some(ok) = Instant::now().checked_sub(std::time::Duration::from_nanos(nanos as u64))
        {
            Ok(SerializableInstant::new(ok))
        } else {
            Ok(SerializableInstant::now())
        }
    }
}

// Implement Deref and DerefMut for convenient access
impl std::ops::Deref for SerializableInstant {
    type Target = Instant;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SerializableInstant {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// Conversion methods
impl From<Instant> for SerializableInstant {
    fn from(instant: Instant) -> Self {
        SerializableInstant(instant)
    }
}

impl From<SerializableInstant> for Instant {
    fn from(serializable: SerializableInstant) -> Self {
        serializable.0
    }
}
