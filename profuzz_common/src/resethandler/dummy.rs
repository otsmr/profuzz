use profuzz_core::traits::ResetHandler;

/// Dummy reset handler which does do nothing
pub struct DummyResetHandler();
impl ResetHandler for DummyResetHandler {
    async fn reset(&mut self) -> Result<(), profuzz_core::error::ProFuzzError> {
        tracing::error!("dummy reset handler triggered, but it does nothing");
        Ok(())
    }
}
