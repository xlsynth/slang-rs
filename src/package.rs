// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::{ConstantValue, Error, Type};

/// Evaluated value parameters, localparams, and resolved typedefs in a package.
///
/// ```
/// use slang_rs::{Compilation, SlangConfig, Source};
/// let config = SlangConfig {
///     sources: &[Source::text("constants.sv",
///         "package p; localparam int answer = 42; endpackage")],
///     ..Default::default()
/// };
/// let packages = Compilation::new(&config).unwrap().packages().unwrap();
/// let answer: i32 = (&packages["p"].parameters["answer"]).try_into().unwrap();
/// assert_eq!(answer, 42);
/// ```
#[derive(Debug, PartialEq)]
pub struct Package {
    pub name: String,
    /// Value parameters and localparams. Type aliases are available in `types`.
    pub parameters: HashMap<String, ConstantValue>,
    /// Resolved typedefs, keyed by their name within this package.
    ///
    /// Each unsupported type carries its own diagnostic, so it does not prevent
    /// access to the package's constants or other types. Results are owned and
    /// remain valid after the compilation is dropped.
    pub types: HashMap<String, Result<Type, Error>>,
}
