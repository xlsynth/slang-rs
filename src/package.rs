// SPDX-License-Identifier: Apache-2.0

use std::{borrow::Cow, collections::HashMap, ops::Index};

use crate::{ConstantValue, Error, IntegerValue, Type};

/// Read a direct member of a live package view or an owned package snapshot.
/// A lookup returns `None` if the package has no member of the requested kind
/// with that name. An unrepresentable type or evaluation error is an `Err`.
/// Live lookups return owned data; snapshot lookups borrow the stored data.
pub trait PackageLookup {
    /// Read the evaluated value of a parameter or localparam.
    fn parameter(&self, name: &str) -> Result<Option<Cow<'_, ConstantValue>>, Error>;

    /// Read an enum constant declared directly in this package.
    fn enumerator(&self, name: &str) -> Result<Option<Cow<'_, IntegerValue>>, Error>;

    /// Resolve a typedef declared in this package.
    fn ty(&self, name: &str) -> Result<Option<Cow<'_, Type>>, Error>;
}

/// Evaluated parameters, localparams, enum constants, and resolved typedefs in a package.
///
/// ```
/// use slang_rs::{Compilation, SlangConfig, Source};
/// let config = SlangConfig {
///     sources: &[Source::text(
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
    /// Enum constants declared directly in this package, keyed by name.
    /// Imported and re-exported enumerators are not included.
    pub enumerators: HashMap<String, IntegerValue>,
    /// Resolved typedefs, keyed by their name within this package.
    ///
    /// Each unsupported type carries its own diagnostic, so it does not prevent
    /// access to the package's constants or other types. Results are owned and
    /// remain valid after the compilation is dropped.
    pub types: HashMap<String, Result<Type, Error>>,
}

impl PackageLookup for Package {
    fn parameter(&self, name: &str) -> Result<Option<Cow<'_, ConstantValue>>, Error> {
        Ok(self.parameters.get(name).map(Cow::Borrowed))
    }

    fn enumerator(&self, name: &str) -> Result<Option<Cow<'_, IntegerValue>>, Error> {
        Ok(self.enumerators.get(name).map(Cow::Borrowed))
    }

    fn ty(&self, name: &str) -> Result<Option<Cow<'_, Type>>, Error> {
        self.types
            .get(name)
            .map(|ty| ty.as_ref().map(Cow::Borrowed).map_err(Clone::clone))
            .transpose()
    }
}

impl Package {
    /// Borrow a value parameter from this snapshot, or return `None` if absent.
    pub fn get(&self, name: &str) -> Option<&ConstantValue> {
        self.parameters.get(name)
    }
}

impl Index<&str> for Package {
    type Output = ConstantValue;

    fn index(&self, name: &str) -> &Self::Output {
        &self.parameters[name]
    }
}
