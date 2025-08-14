/// contains the fuzzing engine which starts the main fuzzing loop and the TUI
pub mod engine;

/// Contains all statistical data collected by the fuzzing engine. If used with a TUI they will be
/// displayed there.
pub mod stats;

mod ui;
