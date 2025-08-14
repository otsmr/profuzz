use crate::error::ProFuzzError;
use crate::mutator::Mutator;

/// Convert the Mutable corpus to `Vec<u8>` or converts `Vec<u8>` to the Mutable
pub trait Corpus
where
    Self: Sized,
{
    /// Parses the bytes to the Mutable
    fn from_bytes(buf: Vec<u8>) -> Option<Self>;
    /// Returns the bytes of the corpus
    fn to_bytes(self) -> Vec<u8>;
    /// Builds the corpus eq. build the checksums and then return the bytes
    fn build(self) -> Vec<u8>;
    /// Returns a human readable representation of the corpus
    fn show(&self) -> String;
}

/// Transport layer to connect to the target
pub trait Transport {
    /// The title shown in the TUI
    fn title(&self) -> String;

    /// Creates a new instance of the transport layer
    /// # Errors
    fn connect(&mut self) -> impl std::future::Future<Output = Result<(), ProFuzzError>>;

    /// Closes the current connection to the target
    /// # Errors
    fn close(&mut self) -> impl std::future::Future<Output = Result<(), ProFuzzError>>;

    /// Read data from the target.
    /// - If no error happen the function must return the lenght that was read from the target.
    /// - If the length is 0 this means that the connection was successfully closed (e.g. for TCP a TCP RST was reseved)
    /// # Errors
    fn read(
        &mut self,
        buf: &mut [u8],
    ) -> impl std::future::Future<Output = Result<usize, ProFuzzError>>;

    /// Write data to the target.
    /// # Errors
    fn write(&mut self, buf: &[u8]) -> impl std::future::Future<Output = Result<(), ProFuzzError>>;
}

/// The reset is called from the protocol fuzzer in case the healthcheck returns the target is not
/// ok.
pub trait ResetHandler {
    /// Function to reset the target in case of an crash
    /// # Errors
    fn reset(&mut self) -> impl std::future::Future<Output = Result<(), ProFuzzError>>;
}

/// Implements a healt check function to check if the target does work correctly
pub trait HealthCheck {
    /// Returns true if the target does work correctly and false if not
    fn is_ok(&mut self) -> impl std::future::Future<Output = Result<bool, ProFuzzError>>;
}

/// Implements the logic to mutate the corpus files
pub trait Mutable {
    /// Mutate is called by the core to mutate the corpus files
    fn mutate<R: rand::Rng>(&mut self, mutator: &mut Mutator<R>);
}
