// SPDX-License-Identifier: Apache-2.0

use std::ops::Index;

use indexmap::IndexMap;

#[derive(Debug, PartialEq)]
pub struct Instance {
    pub def_name: String,
    /// The unescaped instance identifier, without array indices.
    pub inst_name: String,
    /// The full hierarchical path formatted by Slang, including escaped
    /// identifiers, generate scopes, and instance array indices.
    pub path: String,
    /// Child instances keyed by parent-relative path, in Slang's traversal order.
    pub contents: IndexMap<String, Instance>,
}

impl Instance {
    /// Iterate over child instances in Slang's traversal order.
    pub fn values(&self) -> indexmap::map::Values<'_, String, Instance> {
        self.contents.values()
    }

    /// Find an immediate child by its path relative to this instance.
    ///
    /// Include array indices and generate scopes, such as `"lanes[0].u"`.
    /// Paths use Slang's spelling, including escaped identifiers.
    /// Returns `None` for empty or missing paths.
    pub fn child(&self, relative_path: &str) -> Option<&Instance> {
        if relative_path.is_empty() {
            return None;
        }
        self.contents.get(relative_path)
    }
}

impl<'a> IntoIterator for &'a Instance {
    type Item = &'a Instance;
    type IntoIter = indexmap::map::Values<'a, String, Instance>;

    fn into_iter(self) -> Self::IntoIter {
        self.values()
    }
}

impl Index<&str> for Instance {
    type Output = Instance;

    fn index(&self, relative_path: &str) -> &Self::Output {
        self.child(relative_path)
            .unwrap_or_else(|| panic!("no child {relative_path:?} in instance {:?}", self.path))
    }
}

impl Index<usize> for Instance {
    type Output = Instance;

    fn index(&self, index: usize) -> &Self::Output {
        &self.contents[index]
    }
}
