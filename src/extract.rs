// SPDX-License-Identifier: Apache-2.0

//! Typed module interfaces.

use crate::Type;
use std::collections::HashMap;
use std::ops::{Deref, Index};

#[derive(Debug, PartialEq)]
pub enum PortDir {
    Input,
    Output,
    InOut,
}

#[derive(Debug, PartialEq)]
pub struct Port {
    pub dir: PortDir,
    pub name: String,
    pub ty: Type,
}

/// Ports in Slang's port-list order, accessible by name or position.
///
/// Unnamed ports are preserved and can be accessed by position or iteration.
#[derive(Debug, PartialEq)]
pub struct PortList {
    ports: Vec<Port>,
    names: HashMap<String, usize>,
}

impl PortList {
    /// Look up a named port. Returns `None` for empty or missing names.
    ///
    /// Lookup takes expected constant time. If names repeat, returns the first.
    pub fn get_by_name(&self, name: &str) -> Option<&Port> {
        self.names.get(name).map(|&index| &self.ports[index])
    }

    /// Borrow all ports in their original order.
    pub fn as_slice(&self) -> &[Port] {
        &self.ports
    }
}

impl From<Vec<Port>> for PortList {
    fn from(ports: Vec<Port>) -> Self {
        let mut names = HashMap::with_capacity(ports.len());
        for (index, port) in ports.iter().enumerate() {
            if !port.name.is_empty() {
                names.entry(port.name.clone()).or_insert(index);
            }
        }
        Self { ports, names }
    }
}

impl Deref for PortList {
    type Target = [Port];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl Index<usize> for PortList {
    type Output = Port;

    fn index(&self, index: usize) -> &Self::Output {
        &self.ports[index]
    }
}

impl Index<&str> for PortList {
    type Output = Port;

    fn index(&self, name: &str) -> &Self::Output {
        self.get_by_name(name)
            .unwrap_or_else(|| panic!("no port named {name:?}"))
    }
}

impl<'a> IntoIterator for &'a PortList {
    type Item = &'a Port;
    type IntoIter = std::slice::Iter<'a, Port>;

    fn into_iter(self) -> Self::IntoIter {
        self.ports.iter()
    }
}

impl IntoIterator for PortList {
    type Item = Port;
    type IntoIter = std::vec::IntoIter<Port>;

    fn into_iter(self) -> Self::IntoIter {
        self.ports.into_iter()
    }
}

/// The name and resolved type of a value parameter or localparam, without its value.
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

/// A module's parameter declarations and the package imports in scope at its header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleParameterDeclarations {
    /// Number of value and type parameters in [`Compilation::parameter_state_for`](crate::Compilation::parameter_state_for),
    /// including parameter-port localparams. Counting declarations does not evaluate them.
    pub parameter_count: usize,
    /// Names declared by the retained parameter port list and module-body declarations, in source order.
    /// Includes parameter-port localparams and type parameters, but excludes body-local parameters.
    pub declaration_names: Vec<String>,
    /// Effective SystemVerilog time unit and precision (for example, `1ns / 1ps`).
    /// None when neither the source nor the compilation supplies a timescale.
    pub timescale: Option<String>,
    /// Complete package import statements from the module header, in source order.
    pub imports: Vec<String>,
    /// The complete parameter port list, including `#(...)`, localparams, and type parameters.
    /// None when the source has no parameter port list.
    pub port_list: Option<String>,
    /// Overridable parameter declarations originally written inside the module, with
    /// preceding body imports in source order. Place this text after the port declarations.
    pub body: String,
    /// Overridable value parameters in declaration order, keyed by their unescaped names.
    pub parameters: indexmap::IndexMap<String, ParameterDeclaration>,
}

/// Source declaration of one overridable value parameter, before instance overrides.
/// The SystemVerilog fragments are relative to the declaring module's scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterDeclaration {
    /// Type syntax preceding the name, or None when the declaration infers its type.
    pub data_type: Option<String>,
    /// Unpacked dimensions following the name, or an empty string when absent.
    pub dimensions: String,
    /// Default expression in the declaration, or None when a value must be supplied.
    pub default: Option<String>,
    /// Byte range in the containing `port_list` for replacing this parameter's full initializer.
    /// An absent initializer has an empty insertion range after the name and dimensions.
    /// Body parameters have no range. Replacement text should include the leading ` = `.
    pub port_list_initializer: Option<std::ops::Range<usize>>,
    /// Byte range of the full initializer in `body` for a body-declared parameter.
    pub body_initializer: Option<std::ops::Range<usize>>,
    /// Package and typedef identifiers when the entire declared type resolves to a package alias.
    /// Use these names to qualify an imported type when emitting it in another scope.
    pub package_type: Option<(String, String)>,
}

/// Effective values and types of a module's overridable parameters and parameter-port localparams.
#[derive(Clone, Debug, PartialEq)]
pub struct ModuleParameterState {
    /// Value parameters and their resolved types in declaration order.
    pub values: indexmap::IndexMap<String, EvaluatedParameter>,
    /// Resolved type parameters, including any extraction error for an unsupported type.
    pub types: indexmap::IndexMap<String, Result<Type, crate::Error>>,
}

/// One explicitly selected, elaborated value parameter and its resolved type.
/// A type unsupported by the owned type representation is reported separately
/// so callers can still inspect the value.
#[derive(Clone, Debug, PartialEq)]
pub struct EvaluatedParameter {
    /// The value as evaluated by Slang, including packed four-state integers.
    pub value: crate::ConstantValue,
    /// Whether this packed value has SystemVerilog implicit-string conversion behavior.
    pub is_implicit_string: bool,
    pub ty: Result<Type, crate::Error>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IntegralKind;

    #[test]
    fn duplicate_and_unnamed_ports_keep_their_positions() {
        let port = |name: &str, width| Port {
            name: name.into(),
            dir: PortDir::Input,
            ty: Type::Integral {
                kind: IntegralKind::Logic,
                signed: false,
                four_state: true,
                packed_dimensions: vec![],
                unpacked_dimensions: vec![],
                bit_width: Some(width),
            },
        };
        let ports = PortList::from(vec![
            port("data", 1),
            port("", 2),
            port("data", 3),
            port("", 4),
        ]);

        assert_eq!(ports.len(), 4);
        assert!(std::ptr::eq(&ports["data"], &ports[0]));
        assert!(ports.get_by_name("").is_none());
        assert!(ports.get_by_name("missing").is_none());
        assert!(ports.get(4).is_none());
        assert_eq!(
            ports
                .into_iter()
                .map(|p| p.ty.width().unwrap())
                .collect::<Vec<_>>(),
            [1, 2, 3, 4]
        );
    }
}
