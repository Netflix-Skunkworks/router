//! Error types

use std::error;
use std::fmt;

use crate::graphql;

/// The timeout elapsed.
#[derive(Debug, Default)]
pub struct Elapsed;

impl Elapsed {
    /// Construct a new elapsed error
    pub fn new() -> Self {
        Elapsed {}
    }
}

impl fmt::Display for Elapsed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("request timed out")
    }
}

impl From<Elapsed> for graphql::Error {
    fn from(_: Elapsed) -> Self {
        graphql::Error::builder()
            .message(String::from("Request timed out"))
            .extension_code("REQUEST_TIMEOUT")
            .build()
    }
}

impl error::Error for Elapsed {}
