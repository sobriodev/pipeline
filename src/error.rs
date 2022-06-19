//! Error utilities.

use log::{debug, error};
use std::error::Error;
use std::fmt::{Display, Formatter};

/// Shortcut result type for convenience.
pub type Result<T> = std::result::Result<T, Pipeline>;

/// Represents a pipeline error.
#[derive(Debug)]
pub struct Pipeline {
    error_string: String,
    debug_string: Option<String>,
}

impl Pipeline {
    /// Construct a pipeline error instance.
    #[must_use]
    pub fn new(error_string: &str) -> Self {
        Self {
            error_string: error_string.to_string(),
            debug_string: None,
        }
    }

    /// Constructs pipeline error instance with an extra debug string.
    #[must_use]
    pub fn new_debug(error_string: &str, debug_string: &str) -> Self {
        Self {
            error_string: error_string.to_string(),
            debug_string: Some(debug_string.to_string()),
        }
    }

    /// Print pipeline error internals.
    pub fn print_verbose(&self) {
        error!("{}", self.error_string);
        if let Some(dbg_str) = self.debug_string.as_ref() {
            debug!("{}", dbg_str);
        }
    }
}

impl Display for Pipeline {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error_string)
    }
}

impl Error for Pipeline {}
