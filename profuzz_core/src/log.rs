use tracing::level_filters::LevelFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;
use tracing_subscriber::filter::Filtered;
use tracing_subscriber::layer::Layered;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::reload;
use tracing_subscriber::util::SubscriberInitExt;
use tui_logger::TuiTracingSubscriberLayer;

/// Manual manage the logging behavior of profuzz
#[derive(Clone)]
pub struct Logger {
    level_filter: LevelFilter,
    tui_reload_handle: tracing_subscriber::reload::Handle<
        Filtered<TuiTracingSubscriberLayer, tracing::level_filters::LevelFilter, Registry>,
        Registry,
    >,
    // // if you have a better idea go for it!!
    #[allow(clippy::complexity)]
    fmt_reload_handle: tracing_subscriber::reload::Handle<
        Filtered<
            tracing_subscriber::fmt::Layer<
                Layered<
                    tracing_subscriber::reload::Layer<
                        Filtered<
                            TuiTracingSubscriberLayer,
                            tracing::level_filters::LevelFilter,
                            Registry,
                        >,
                        Registry,
                    >,
                    Registry,
                >,
            >,
            tracing::level_filters::LevelFilter,
            Layered<
                tracing_subscriber::reload::Layer<
                    Filtered<
                        TuiTracingSubscriberLayer,
                        tracing::level_filters::LevelFilter,
                        Registry,
                    >,
                    Registry,
                >,
                Registry,
            >,
        >,
        Layered<
            tracing_subscriber::reload::Layer<
                Filtered<TuiTracingSubscriberLayer, tracing::level_filters::LevelFilter, Registry>,
                Registry,
            >,
            Registry,
        >,
    >,
}

impl Logger {
    /// Initialized the logger. This allows to collect all logs and show then in the TUI if
    /// enabled.
    /// # Panics
    #[must_use]
    pub fn init(verbose: bool) -> Self {
        let mut level_filter = LevelFilter::INFO;
        if verbose {
            level_filter = LevelFilter::DEBUG;
        }

        let fmt_logger = tracing_subscriber::fmt::Layer::new().with_filter(level_filter);
        let (fmt_logger, fmt_reload_handle) = reload::Layer::new(fmt_logger);

        let tui_logger = tui_logger::TuiTracingSubscriberLayer.with_filter(LevelFilter::OFF);
        let (tui_logger, tui_reload_handle) = reload::Layer::new(tui_logger);

        tracing_subscriber::registry()
            .with(tui_logger)
            .with(fmt_logger)
            .init();

        tui_logger::init_logger(tui_logger::LevelFilter::max()).expect("");

        Self {
            level_filter,
            tui_reload_handle,
            fmt_reload_handle,
        }
    }
    pub(crate) fn enable_tui(&self) {
        let _ = self
            .fmt_reload_handle
            .modify(|layer| *layer.filter_mut() = LevelFilter::OFF);
        let _ = self
            .tui_reload_handle
            .modify(|layer| *layer.filter_mut() = self.level_filter);
    }
    pub(crate) fn disable_tui(&self) {
        let _ = self
            .fmt_reload_handle
            .modify(|layer| *layer.filter_mut() = self.level_filter);
        let _ = self
            .tui_reload_handle
            .modify(|layer| *layer.filter_mut() = LevelFilter::OFF);
    }
}
