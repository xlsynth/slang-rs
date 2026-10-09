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
