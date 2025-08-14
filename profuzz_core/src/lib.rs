//!
//! `profuzz` is a generic approach to easily create a fast and easy-to use network protocol fuzzer for custom targets.
//!
//! `profuzz` aims to be used mainly in the embedded world, where most of the time it is not easy to create a
//! running harness on a Linux-based system because of hardware dependencies, the source code is not available,
//! or it requires hardware attacks to dump the firmware. Dumping the firmware, reverse engineering, and then
//! fuzzing potential targets is time intensive. In these cases `profuzz` can be used to find "low-hanging"
//! fruits by fuzzing either the network stack itself or custom binary protocols.
//!
//! To use the `profuzz_core` you have to implement the following traits:
//! - `Healthcheck`: Used to verifiy to correct working of the target
//! - `Transport`: Used to send and receive messages to the target
//! - `Mutate`: Used to mutate the corpus files
//! - `ResetHandler`: Used to reset the target in case it did crash
//!
//! You can find common implementation ready to use in the [profuzz_common](https://github.com/otsmr/profuzz/blob/main/profuzz_common) crate.
//!
//! The following code shows the basic setup required to create a network stack fuzzer. It uses a TCP server on
//! the target to detect a crash and sends the packets directly on eth0.
//!
//! `profuzz_core` can be configured and started either in headless mode or using `start_cli` allowing the user
//! to configure the different options using CLI.
//!
//! In case you want to use `profuzz` in headless mode, please have a look into the cli.rs to see
//! how to set the fuzzing engine up directly.
//!
//! The full example can be found in the [example](https://github.com/otsmr/profuzz/tree/main/example) folder.
//!
//! ```rs
//! #[tokio::main]
//! async fn main() {
//!     // Defining the Transport layer, in this case a raw linux socket.
//!     let transport = RawSocketTransport::new("eth0");
//!
//!     // Defining the Healthcheck to detect a crash
//!     let healthcheck = TcpHealthcheck::new(
//!        "lo0",
//!        TcpPacket {
//!            eth_src: MacAddr::from_str("32:a4:e7:9a:c7:99").unwrap(),
//!            eth_dst: MacAddr::from_str("32:a4:e7:9a:c7:8a").unwrap(),
//!            vlan_id: None,
//!            ipv4_src: Ipv4Addr::from([127, 0, 0, 2]),
//!            ipv4_dst: Ipv4Addr::from([127, 0, 0, 1]),
//!            dport: 1337,
//!            sport: 1330,
//!        },
//!    )
//!    .unwrap();
//!
//!     // Setting up the protocol fuzzer with the different implementations
//!     let fuzzer = ProFuzzBuilder::new(transport, healthcheck, DummyResetHandler());
//!
//!     // Starting the CLI including a TUI, and defining the `Mutable` implementation struct that
//!     // implements the mutation of the corpus files
//!     if let Err(err) = fuzzer.start_cli::<EtherMutatorOwned>().await {
//!         eprintln!("{err}");
//!     }
//! }
//! ```
//!

#![deny(missing_docs)]
#![deny(unsafe_code, clippy::unwrap_used)]
#![warn(clippy::pedantic)]

/// Contains all `traits` that have to be defined to use `profuzz_core`. There are common
/// implementations in the `profuzz_common` crate that could be used.
pub mod traits;

/// Contains the fuzzing engine and the TUI which can also be used in a headless mode.
pub mod fuzz;

/// Manage the logger. This can be used to initialize the `tracing_subscriber` in case
/// `profuzz_core` is used in headless mode.
pub mod log;

/// Output directory of the fuzzer
pub mod output;

/// Triage support for identifying the crash can be started via the cli or through the headless
/// mode.
pub mod triage;

/// Contains all the different errors `profuzz_core` can return.
pub mod error;

/// Mutation engine which can be used to mutate numbers and bytes.
pub mod mutator;

/// Contains the `ProFuzzBuilder` to start `profuzz_core` in CLI mode.
pub mod cli;

mod dangerous_numbers;

/// A internal collection of different types
pub(crate) mod types;
