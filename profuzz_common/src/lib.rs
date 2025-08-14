//! Collection of common implementations for `profuzz_core`
//!
//! `profuzz_common` is a collection of ready to use implementations for the different `traits` to be implemented to run [profuzz_core](https://github.com/otsmr/profuzz/).

//! Currently there are the following common implementations:

//! - `Healthcheck`
//!     - `TcpHealthcheck`: This can be used when the target has listening TCP port.
//!     - `DummyHealthcheck`: Always returns true.
//! - `Mutable`
//!     - `EtherMutatorOwned`: Implements the mutation for various network packets.
//! - `ResetHandler`
//!     - `DummyResetHandler`: Does nothing.
//! - `Transport`
//!     - `TcpTransport`: Connects to a TCP server and sends the fuzzing input over TCP.
//!     - `RawSocketTransport`: Sends the fuzzing input raw on the given interface.
//!
//!
#![deny(missing_docs)]
#![deny(unsafe_code, clippy::unwrap_used)]
#![warn(clippy::pedantic)]

/// A collection of differed `Transport` implementations
pub mod transport;

/// A collection of differed `ResetHandler` implementations
pub mod resethandler;

/// A collection of differed `HealthCheck` implementations
pub mod healthcheck;

/// A collection of differed `Mutable` implementations
pub mod mutable;
