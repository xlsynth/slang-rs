// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::ops::Index;
use std::str::FromStr;

#[derive(Debug, PartialEq)]

pub struct Parameter {
    pub name: String,
    pub value: String,
}

impl Parameter {
    /// Parse the parameter's `value` into any type that implements [`FromStr`].
    ///
    /// # Type Parameters
    ///
    /// * **`T`** – The numeric type you want (`i64`, `u128`,
    ///   [`num_bigint::BigInt`], [`num_bigint::BigUint`], `f64`, ...).
    ///   `T` only needs to satisfy `T: FromStr`.
    ///
    /// # Errors
    ///
    /// Returns `Err(T::Err)` if `value` is *not* a valid textual
    /// representation for `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use num_bigint::{BigInt, BigUint};
    /// use std::convert::TryFrom;
    ///
    /// # use slang_rs::Parameter;
    /// let p = Parameter { name: "answer".into(), value: "42".into() };
    ///
    /// // Primitive integer (type inferred)
    /// let n: i32 = p.parse().unwrap();
    ///
    /// // Explicit turbofish when inference can’t decide
    /// let n128 = p.parse::<u128>().unwrap();
    ///
    /// // Big integers
    /// let big:  BigInt  = p.parse().unwrap();
    /// let ubig: BigUint = p.parse().unwrap();
    /// ```
    pub fn parse<T>(&self) -> Result<T, T::Err>
    where
        T: FromStr,
    {
        self.value.parse()
    }
}

#[derive(Debug, PartialEq)]
pub struct Package {
    pub name: String,
    pub parameters: HashMap<String, Parameter>,
}

impl Index<&str> for Package {
    type Output = Parameter;

    fn index(&self, key: &str) -> &Self::Output {
        &self.parameters[key]
    }
}

pub fn extract_packages(
    cfg: &crate::SlangConfig,
) -> Result<HashMap<String, Package>, Box<dyn Error>> {
    Ok(extract_packages_from_value(&crate::run_slang(cfg)?))
}

pub fn extract_packages_from_value(value: &Value) -> HashMap<String, Package> {
    let mut packages = HashMap::new();

    if let Some(members) = value
        .get("design")
        .and_then(|v| v.get("members").and_then(|v| v.as_array()))
    {
        for member in members {
            if let Some(kind) = member.get("kind") {
                if kind == "CompilationUnit" {
                    extract_packages_from_compilation_unit(member, &mut packages);
                }
            }
        }
    }

    packages
}

fn extract_packages_from_compilation_unit(value: &Value, packages: &mut HashMap<String, Package>) {
    if let Some(members) = value.get("members").and_then(|v| v.as_array()) {
        for member in members {
            if let Some(kind) = member.get("kind") {
                if kind == "Package" {
                    if let Some(name) = member.get("name").and_then(|v| v.as_str()) {
                        let mut package = Package {
                            name: name.to_string(),
                            parameters: HashMap::new(),
                        };
                        if let Some(members) = member.get("members").and_then(|v| v.as_array()) {
                            for member in members {
                                if let Some(kind) = member.get("kind") {
                                    if kind == "Parameter" {
                                        if let Some(parameter) = process_parameter(member) {
                                            package
                                                .parameters
                                                .insert(parameter.name.clone(), parameter);
                                        }
                                    }
                                }
                            }
                        }
                        packages.insert(name.to_string(), package);
                    }
                }
            }
        }
    }
}

fn process_parameter(member: &Value) -> Option<Parameter> {
    if let Some(value) = member.get("value").and_then(|v| v.as_str()) {
        if let Some(name) = member.get("name").and_then(|v| v.as_str()) {
            return Some(Parameter {
                name: name.to_string(),
                value: value.to_string(),
            });
        }
    }
    None
}
