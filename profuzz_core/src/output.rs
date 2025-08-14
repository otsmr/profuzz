//! The output directory contains the following files and folders:
//!
//! - `/crashes/`: All buffers which probably crashed the target.
//! - `crashes.json`: Crash file which stores all informations to reproduce a potential crash
//! - `stats.json`: Statistics which are shown in the TUI
//! - `queue.json`: Current queue files which are used to crate new mutations
//!

use crate::error::{ProFuzzError, ProFuzzResult};
use std::path::PathBuf;

/// Manages the output directory like creating the needed folders,
/// resuming a fuzzing session or storing informations
#[derive(Clone)]
pub struct Output {
    path: PathBuf,
}

impl Output {
    /// create a new output directory instance
    /// # Errors
    pub fn init(path: PathBuf, auto_resume: bool) -> ProFuzzResult<Self> {
        if path.is_dir() {
            if !auto_resume {
                return Err(ProFuzzError::AutoResumeNotEnabled);
            }
        } else {
            tracing::info!("Created output directory: {}", path.display());
            std::fs::create_dir_all(&path)?;
        }

        std::fs::create_dir_all(path.join("crashes"))?;

        Ok(Self { path })
    }

    pub(crate) fn get_crash_file(&self) -> PathBuf {
        self.path.join("crashes.json")
    }
    pub(crate) fn get_crash_folder(&self) -> PathBuf {
        let dir = self.path.join("crashes");
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|_| panic!("Could not create crashes folder: {}!", dir.display()));
        dir
    }

    // pub(crate) fn get_queue_file(&self) -> PathBuf {
    //     self.path.join("queue.json")
    // }
    // pub(crate) fn get_stats_file(&self) -> PathBuf {
    //     self.path.join("stats.json")
    // }
}
