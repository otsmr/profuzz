use crate::error::ProFuzzError;
use crate::fuzz::engine::FuzzEngine;
use crate::log::Logger;
use crate::output::Output;
use crate::traits::{Corpus, HealthCheck, Mutable, ResetHandler, Transport};
use crate::triage::dynamic::DynamicTriage;
use clap::Parser;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct ProFuzzCliArgs {
    /// Verbose mode
    #[arg(long, default_value_t = false)]
    verbose: bool,
    #[command(subcommand)]
    command: ProFuzzCliCommands,
}

#[derive(Debug, Subcommand)]
enum ProFuzzCliCommands {
    /// Triage found crashes to identify the potential root cause
    Triage {
        /// output directory for fuzzer findings
        #[arg(long, short)]
        out_dir: PathBuf,
    },
    // Starts the fuzzing loop
    Fuzz {
        /// input directory with test cases
        // (or '-' to resume, also see PROFUZZ_AUTORESUME)
        #[arg(long, short)]
        in_dir: PathBuf,
        /// output directory for fuzzer findings
        #[arg(long, short)]
        out_dir: PathBuf,
        /// Displays the profuzz UI
        #[arg(long, default_value_t = false)]
        hide_ui: bool,
        /// If output directory is not empty auto resume the session
        #[arg(long, default_value_t = false)]
        auto_resume: bool,
    },
}

/// A helper struct to setup the CLI application or to start the fuzzer or triaging with fewer
/// lines of code.
pub struct ProFuzzBuilder<T: Transport, H: HealthCheck, R: ResetHandler> {
    transport: T,
    healthcheck: H,
    resethandler: R,
}

impl<T: Transport, H: HealthCheck, R: ResetHandler> ProFuzzBuilder<T, H, R> {
    /// Generates a new instance of `ProFuzz`
    pub fn new(transport: T, healthcheck: H, resethandler: R) -> Self {
        Self {
            transport,
            healthcheck,
            resethandler,
        }
    }
}

impl<T: Transport, H: HealthCheck, R: ResetHandler> ProFuzzBuilder<T, H, R> {
    /// Starts the `ProFuzzer` as a CLI application
    /// # Errors
    pub async fn start_cli<M>(self) -> Result<(), ProFuzzError>
    where
        M: Corpus + Mutable + Clone,
    {
        let args = ProFuzzCliArgs::parse();
        let logger = Logger::init(args.verbose);
        match args.command {
            ProFuzzCliCommands::Triage { out_dir } => {
                let output = Output::init(out_dir, true)?;
                let mut triager =
                    DynamicTriage::new(self.transport, self.healthcheck, self.resethandler);
                triager.triage_from_output_dir::<M>(&output).await?;
                Ok(())
            }
            ProFuzzCliCommands::Fuzz {
                in_dir,
                out_dir,
                hide_ui,
                auto_resume,
            } => {
                let output = Output::init(out_dir, auto_resume)?;
                let mut fuzzengine: FuzzEngine<M, _, _, _> =
                    FuzzEngine::new(self.transport, self.healthcheck, self.resethandler, output);
                fuzzengine.fuzz(!hide_ui, &in_dir, Some(logger)).await
            }
        }
    }
}
