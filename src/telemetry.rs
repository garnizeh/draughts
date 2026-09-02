//! Observability — §21.
//!
//! Structured logging, initialised before anything that can fail, so that a
//! startup refusal is a log line rather than a silent exit code.

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// The default filter. Overridden by `RUST_LOG`.
const DEFAULT_FILTER: &str = "info,draughts=debug,tower_http=info";

/// Initialise logging.
///
/// `json` is for a machine reading the output; the human-readable form is the
/// default because the deployment target is one machine with one operator
/// (§22.1).
pub fn init(json: bool) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));

    let registry = tracing_subscriber::registry().with(filter);

    if json {
        registry
            .with(fmt::layer().json().with_current_span(true))
            .init();
    } else {
        registry.with(fmt::layer().with_target(true)).init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_filter_parses() {
        assert!(EnvFilter::try_new(DEFAULT_FILTER).is_ok());
    }
}
