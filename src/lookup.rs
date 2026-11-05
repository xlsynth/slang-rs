// SPDX-License-Identifier: Apache-2.0

use crate::Error;
use std::fmt;

/// Why a named top, package, or package member could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookupError {
    /// No object of the requested kind has this name in the queried scope.
    NotFound { kind: &'static str, name: String },
    /// Compilation, evaluation, or conversion of the result failed.
    Extraction(Error),
}

impl LookupError {
    /// Identify the missing object; package members use their qualified name.
    pub(crate) fn not_found(kind: &'static str, name: impl Into<String>) -> Self {
        Self::NotFound {
            kind,
            name: name.into(),
        }
    }
}

impl From<Error> for LookupError {
    fn from(error: Error) -> Self {
        Self::Extraction(error)
    }
}

impl fmt::Display for LookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { kind, name } => write!(f, "{kind} '{name}' not found"),
            Self::Extraction(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for LookupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotFound { .. } => None,
            Self::Extraction(error) => Some(error),
        }
    }
}
