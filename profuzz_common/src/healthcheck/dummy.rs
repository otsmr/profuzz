use profuzz_core::traits::HealthCheck;

/// Dummy health check does always returns true
pub struct DummyHealthcheck();
impl HealthCheck for DummyHealthcheck {
    async fn is_ok(&mut self) -> Result<bool, profuzz_core::error::ProFuzzError> {
        Ok(true)
    }
}
