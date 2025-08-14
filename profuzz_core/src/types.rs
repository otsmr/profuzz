use crate::error::ProFuzzResult;
use crate::fuzz::stats::Stats;
use crate::output::Output;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
/// This represents a crash that stores all send buffers since the last sucessfull healthcheck.
pub(crate) struct Crash {
    // mutations_since_reseeding: usize,
    pub(crate) buffer: Vec<u8>,
    // state: Option<MutationState>,
    pub(crate) last_send_buffers: Vec<Vec<u8>>,
    pub(crate) stats: Stats,
}

impl Crash {
    pub(crate) fn load(output: &Output) -> ProFuzzResult<Vec<Crash>> {
        let path = output.get_crash_file();
        if !path.is_file() {
            return Ok(vec![]);
        }
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }
    pub(crate) fn save(self, output: &Output) -> ProFuzzResult<()> {
        let path = output.get_crash_file();
        let mut current = Self::load(output)?;
        current.push(self);
        let content = serde_json::to_string(&current)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
