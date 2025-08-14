use crate::error::{ProFuzzError, ProFuzzResult};
use crate::output::Output;
use crate::traits::{Corpus, HealthCheck, Mutable, ResetHandler, Transport};
use crate::triage::utils::{hamming_distance, mark_differences};
use crate::types::Crash;
use sha1::{Digest, Sha1};
use std::time::Duration;
use tokio::time::sleep;

/// Dynamic approach for identifying the crash cause.
/// - This does iterate over all corpus files send since the last successfull health check
/// - Everytime a single corpus is send a healthcheck is performed to determine if this was the cause
pub struct DynamicTriage<T, H, R>
where
    T: Transport,
    H: HealthCheck,
    R: ResetHandler,
{
    transport: T,
    healthcheck: H,
    resethandler: R,
}

impl<T, H, R> DynamicTriage<T, H, R>
where
    T: Transport,
    H: HealthCheck,
    R: ResetHandler,
{
    /// creates a new instance of the dynamic triaging
    pub fn new(transport: T, healthcheck: H, resethandler: R) -> Self {
        Self {
            transport,
            healthcheck,
            resethandler,
        }
    }

    /// loads the crash informations for the output dir and tries to identiy the single input
    /// # Errors
    pub async fn triage_from_output_dir<M: Mutable + Corpus + Clone>(
        &mut self,
        output: &Output,
    ) -> ProFuzzResult<()> {
        let crash_file = output.get_crash_file();
        let content = std::fs::read_to_string(crash_file)?;
        let crashes: Vec<Crash> = serde_json::from_str(&content)?;
        self.triage::<M>(crashes, output).await?;
        Ok(())
    }

    pub(crate) async fn triage<M>(
        &mut self,
        crashes: Vec<Crash>,
        output: &Output,
    ) -> ProFuzzResult<()>
    where
        M: Mutable + Corpus + Clone,
    {
        let len = crashes.len();
        for (i, crash) in crashes.into_iter().enumerate() {
            println!("Triage {i}/{len} [y/n]? ");
            let mut input = String::new();

            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");

            let input = input.trim();

            if input == "y" {
                tracing::info!("Starting with detecting the cause.");
                if !self.find_the_crash_cause::<M>(crash, output).await? {
                    tracing::error!("Could not detect the crash :/");
                }
            }
        }
        Ok(())
    }

    /// Iterates over all `last_send_buffers` items and checking after every send if the target
    /// crashed.
    async fn find_the_crash_cause<M>(
        &mut self,
        mut crash: Crash,
        output: &Output,
    ) -> ProFuzzResult<bool>
    where
        M: Mutable + Corpus + Clone,
    {
        tracing::info!("Starting with a healthcheck.");
        if self
            .send_and_detect_crash::<M>(&crash.buffer, output, &crash.last_send_buffers)
            .await?
        {
            return Ok(true);
        }
        tracing::info!("Running full test of all buffers.");

        let mut similar_corpuses = vec![];
        let total_len = crash.last_send_buffers.len();
        crash.last_send_buffers.reverse();
        for (i, crash_buffer) in crash.last_send_buffers.into_iter().enumerate() {
            print!("\r Testing {i}/{total_len}");
            if self
                .send_and_detect_crash::<M>(&crash_buffer, output, &similar_corpuses)
                .await?
            {
                tracing::info!("Identified the corpus that crashed the target.");
                return Ok(true);
            }
            similar_corpuses.push(crash_buffer);
        }
        Ok(false)
    }

    /// Sends the buffer to the target and verifies if the target crashed.
    /// In case of an crash, the buffer is stored in the output dir.
    async fn send_and_detect_crash<M>(
        &mut self,
        buffer: &[u8],
        output: &Output,
        similar_corpuses: &[Vec<u8>],
    ) -> ProFuzzResult<bool>
    where
        M: Mutable + Corpus + Clone,
    {
        // make sure the target is healthy
        loop {
            let is_ok = self.healthcheck.is_ok().await;
            if is_ok.is_ok_and(|x| x) {
                break;
            }
            sleep(Duration::from_millis(1000)).await;
        }
        let _ = self.transport.connect().await;
        if let Err(err) = self.transport.write(buffer).await {
            tracing::error!("COULD NOT write: {err}");
        }
        let mut tmp = [0; 2000];
        let _ = self.transport.read(&mut tmp).await;
        let _ = self.transport.close().await;

        let mut crashed = false;
        loop {
            let is_ok = self.healthcheck.is_ok().await;

            if !crashed && (is_ok.is_err() || is_ok.as_ref().is_ok_and(|x| !x)) {
                crashed = true;
                tracing::info!("TARGET crashed");

                let Some(base) = M::from_bytes(buffer.to_vec()) else {
                    return Err(ProFuzzError::Custom {
                        err_msg: "Could not create a structured representation of the crash"
                            .to_string(),
                    });
                };

                let mut min_hamming = usize::MAX;
                let mut most_equal = &similar_corpuses[0];

                for non_crash in similar_corpuses {
                    if let Some(hamming) = hamming_distance(non_crash, buffer) {
                        if hamming == 0 {
                            continue;
                        }
                        if hamming < min_hamming {
                            most_equal = non_crash;
                            min_hamming = hamming;
                        }
                    }
                }

                if let Some(most_equal) = M::from_bytes(most_equal.to_owned()) {
                    let marked = mark_differences(&base.show(), &most_equal.show());
                    println!("{marked}");
                } else {
                    println!("{}", base.show());
                }

                {
                    let mut hash = Sha1::new();
                    hash.update(buffer);
                    let hash = hash.finalize();
                    let file = output.get_crash_folder().join(hex::encode(hash));
                    if std::fs::write(&file, buffer).is_err() {
                        tracing::error!("Could not store crash in {}", file.display());
                    } else {
                        tracing::info!("Crash stored in {}", file.display());
                    }
                }

                self.resethandler.reset().await?;
            }
            if is_ok.is_ok_and(|x| x) {
                break;
            }
            sleep(Duration::from_millis(1000)).await;
        }
        Ok(crashed)
    }
}
