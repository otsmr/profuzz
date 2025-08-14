use crate::error::ProFuzzError;
use crate::fuzz::stats::{ExecsPerSecond, SerializableInstant, StatsType};
use crate::fuzz::ui::show_ui;
use crate::log::Logger;
use crate::mutator::Mutator;
use crate::output::Output;
use crate::traits::{Corpus, HealthCheck, Mutable, ResetHandler, Transport};
use crate::types::Crash;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use sha1::{Digest, Sha1};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// The main fuzzing engine containing the coordination of the different traits like connecting to
/// the target, performing the healthcheck or reseting the target.
pub struct FuzzEngine<M, T, H, R>
where
    M: Clone + Corpus + Mutable,
    T: Transport,
    H: HealthCheck,
    R: ResetHandler,
{
    transport: T,
    healthcheck: H,
    resethandler: R,
    mutator: Mutator<ChaChaRng>,
    pub(crate) stats: StatsType,
    output: Output,
    queue: VecDeque<QueueElement<M>>,
    last_send_buffers: VecDeque<Vec<u8>>,
    unique_crashes: HashSet<Vec<u8>>,
    unique_responses: HashSet<Vec<u8>>,
    read_buffer: Vec<u8>,
}

struct QueueElement<M> {
    corpus: M,
}

impl<M, T, H, R> FuzzEngine<M, T, H, R>
where
    M: Clone + Corpus + Mutable,
    T: Transport,
    H: HealthCheck,
    R: ResetHandler,
{
    pub(crate) fn new(transport: T, healthcheck: H, resethandler: R, output: Output) -> Self {
        let seed = [0; 32];
        let rng = rand_chacha::ChaChaRng::from_seed(seed);

        Self {
            transport,
            healthcheck,
            resethandler,
            output,
            mutator: Mutator::new(rng),
            unique_responses: HashSet::default(),
            unique_crashes: HashSet::default(),
            last_send_buffers: VecDeque::default(),
            read_buffer: vec![0; 3000],
            stats: Arc::default(),
            queue: VecDeque::default(),
        }
    }

    /// Sends the current copurs to the target.
    /// When a response is returned from the target this is used to create
    /// coverage based new corpus files.
    /// It returnes false if it was not able to reset the current connection
    async fn send_corpus(&mut self, corpus: &M) -> bool {
        let mut backoff_time = 100;
        let mut reset_tried = 0;

        loop {
            if let Ok(mut stats) = self.stats.write() {
                stats.backoff_time = 0;
            }

            if self.transport.connect().await.is_ok() {
                break;
            }
            // tries to connect to the target
            reset_tried += 1;
            if backoff_time <= (1000 * 5) {
                backoff_time *= 2;
            }
            if let Ok(mut stats) = self.stats.write() {
                stats.executions_per_second.clear();
            }
            if let Ok(mut stats) = self.stats.write() {
                stats.backoff_time = backoff_time;
            }

            if reset_tried > 3 {
                return false;
            }
            sleep(Duration::from_millis(backoff_time));
        }

        let bytes = corpus.to_owned().to_bytes();
        tracing::debug!(
            "Sending: {:X?}",
            bytes.iter().take(25).collect::<Vec<&u8>>()
        );

        // Sending fuzzing input to the connected target
        if let Err(err) = self.transport.write(&bytes).await {
            if matches!(err, ProFuzzError::Timeout { .. })
                && let Ok(mut stats) = self.stats.write()
            {
                stats.total_timeouts += 1;
            }
            tracing::warn!("[WRITING] {err}");
            return false;
        }

        // try to read from the target
        match self.transport.read(&mut self.read_buffer).await {
            Ok(size) => {
                if size == 0 {
                    return true; // target closed the connection
                }

                // Got a response -> Check if this is a unique response and if so add the corpus to
                // the queue with the mutation state so it could explore the newly found path even
                // more
                let mut hasher = Sha1::new();
                hasher.update(&self.read_buffer[0..size]);
                let result = hasher.finalize().to_vec();

                if self.unique_responses.insert(result.clone()) {
                    let mut max_info_size = size;
                    let mut trunc = String::new();
                    if max_info_size > 5 {
                        max_info_size = 5;
                        trunc = format!("({max_info_size} of {size} shown)");
                    }

                    tracing::info!(
                        "Got unique response: {:X?}{trunc}",
                        &self.read_buffer[0..max_info_size]
                    );

                    if let Ok(mut stats) = self.stats.write() {
                        stats.total_unique_responses += 1;
                        stats.last_new_path = Some(SerializableInstant::now());
                        stats.corpus_count += 3;

                        self.queue.push_back(QueueElement {
                            corpus: corpus.clone(),
                        });
                    }
                }
                let _ = self.transport.close().await;
            }
            Err(err) => {
                if matches!(err, ProFuzzError::Timeout { .. })
                    && let Ok(mut stats) = self.stats.write()
                {
                    stats.total_timeouts += 1;
                }
                tracing::warn!("[READING]: {err}");
                return false;
            }
        }
        true
    }

    fn load_initial_corpus(in_dir: &PathBuf) -> Result<Vec<M>, ProFuzzError> {
        let corpuses = fs::read_dir(in_dir)?;

        let mut initial_corpus = vec![];

        for corpus in corpuses {
            let corpus = corpus?;
            if !corpus.file_type()?.is_file() {
                continue;
            }
            let file = File::open(corpus.path())?;
            let mut buffer = Vec::new();
            BufReader::new(file).read_to_end(&mut buffer)?;
            if let Some(corpus) = M::from_bytes(buffer) {
                initial_corpus.push(corpus);
            } else {
                tracing::error!("Could not load corpus file: {}", corpus.path().display());
            }
        }
        Ok(initial_corpus)
    }

    async fn do_healthcheck(&mut self, after_reset: bool) -> bool {
        // check for unique crashes...
        let mut failed = false;
        if let Ok(is_ok) = self.healthcheck.is_ok().await {
            if !is_ok {
                failed = true;
            }
        } else {
            failed = true;
        }
        if failed {
            if after_reset {
                return false;
            }
            let len = self.last_send_buffers.len();
            if len > 0
                && let Some(buffer) = self.last_send_buffers.get(len - 1).cloned()
            {
                let crash = Crash {
                    stats: self.stats.read().expect("").clone(),
                    buffer: buffer.clone(),
                    last_send_buffers: self.last_send_buffers.clone().into_iter().collect(),
                };

                if let Err(err) = crash.save(&self.output) {
                    tracing::error!("{err}");
                }

                if self.unique_crashes.insert(buffer)
                    && let Ok(mut ok) = self.stats.write()
                {
                    ok.total_crashes += 1;
                    ok.last_unique_crash = Some(SerializableInstant::now());
                    return false;
                }
            }
            return false;
        }
        if let Ok(mut ok) = self.stats.write() {
            ok.last_healt_check = Some(SerializableInstant::now());
        }
        true
    }

    #[allow(clippy::too_many_lines)]
    /// Starts the main fuzzing loop, and if enabled the TUI.
    /// # Errors
    pub async fn fuzz(
        &mut self,
        enable_ui: bool,
        in_dir: &PathBuf,
        logger: Option<Logger>,
    ) -> Result<(), ProFuzzError> {
        {
            tracing::info!("Performing healthcheck bevore starting.");
            // Test if it is possible to connect to the target
            self.transport.connect().await?;
            self.transport.close().await?;
            if !self.healthcheck.is_ok().await? {
                return Err(ProFuzzError::ConnectionFailed {
                    err_msg: "Initial healthcheck was not successfull. Exiting.".to_owned(),
                });
            }
        }

        // load queue from the output to resume from the old state
        let initial_corpus = Self::load_initial_corpus(in_dir)?;

        if initial_corpus.is_empty() {
            return Err(ProFuzzError::Custom {
                err_msg: "No input corpus found!.".to_owned(),
            });
        }

        let mut ui_handler = None;

        {
            if let Ok(mut stats) = self.stats.write() {
                stats.running = true;
                stats.title = self.transport.title();
                stats.started = Some(SerializableInstant::now());
            }
            ExecsPerSecond::start(self.stats.clone());
        }

        // Spawn a thread for the TUI if enabled
        if enable_ui {
            let stats = self.stats.clone();
            ui_handler = Some(std::thread::spawn(move || {
                if let Some(logger) = &logger {
                    logger.enable_tui();
                }
                show_ui(&stats);
                if let Some(logger) = &logger {
                    logger.disable_tui();
                }
            }));
        }

        tracing::info!("Testing initial corpuse files");

        for init_corpus in &initial_corpus {
            self.send_corpus(init_corpus).await;
        }

        self.queue = VecDeque::from(
            initial_corpus
                .into_iter()
                .map(|x| QueueElement { corpus: x })
                .collect::<Vec<_>>(),
        );

        if let Ok(mut stats) = self.stats.write() {
            stats.corpus_count = self.queue.len();
        }

        let mut running = true;

        // when the transport layer never fails like in case of UDP
        // a health check is triggered every 5 seconds
        // Because a healt check does takes some time this is not triggered every time a message is
        // send
        let mut last_health_check = Instant::now();

        while running {
            if let Ok(mut stats) = self.stats.write() {
                stats.corpus_count = self.queue.len();
            }

            let mut next_cycle = VecDeque::new();

            tracing::info!("Starting new cycle.");

            while let Some(element) = self.queue.pop_front() {
                // each element should be used multiple times as "root" and the mutation should be
                // started from there
                for _ in 0..50 {
                    // Start from the source corpus
                    let mut corpus = element.corpus.clone();
                    // and then mutate this source corpus 100x
                    for _ in 0..1000 {
                        if !running {
                            break;
                        }

                        // if let Some(state) = &element.state {
                        //     self.mutator.set_chances(state.chance.clone());
                        // }

                        corpus.mutate(&mut self.mutator);

                        // store elements to send into a buffer so we can easily reproduce a crash
                        self.last_send_buffers.push_front(corpus.clone().to_bytes());

                        let sending_without_error = self.send_corpus(&corpus).await;

                        let mut after_reset = false;
                        loop {
                            if let Ok(mut stats) = self.stats.write() {
                                stats.total_executions += 1;
                                stats.executions_per_second.add();
                                if !stats.running {
                                    running = false;
                                    break;
                                }
                            }

                            // as a healthcheck does slow down the fuzzing process try to do it not
                            // every time
                            if self.last_send_buffers.len() < 20_000
                                && last_health_check.elapsed().as_secs() <= 4
                                && sending_without_error
                            {
                                break;
                            }

                            if self.do_healthcheck(after_reset).await {
                                last_health_check = Instant::now();
                                // safe also packets which where send before the healthcheck in
                                // case they are also needed
                                self.last_send_buffers.truncate(5_000);
                                // self.last_send_buffers.clear();
                                break;
                            }

                            tracing::info!("Resethandler triggered");

                            // after an reset wait until the healthcheck shows good again
                            after_reset = true;
                            self.resethandler.reset().await?;
                            sleep(Duration::from_secs(1));
                        }
                    }
                }
                // add all send elements to the queue
                next_cycle.push_back(element);
            }

            if let Ok(mut stats) = self.stats.write() {
                stats.cylcles_done += 1;
            }

            self.queue = next_cycle;
        }

        if let Ok(mut stats) = self.stats.write() {
            stats.running = false;
        }
        if let Some(ui_handler) = ui_handler {
            let _ = ui_handler.join();
        }
        Ok(())
    }
}
