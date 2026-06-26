// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub struct Instance {
    pub def_name: String,
    pub inst_name: String,
    pub hier_prefix: String,
    pub contents: Vec<Rc<RefCell<Instance>>>,
}

pub fn extract_hierarchy(
    cfg: &crate::SlangConfig,
) -> Result<HashMap<String, Instance>, Box<dyn Error>> {
    Ok(extract_hierarchy_from_value(&crate::run_slang(cfg)?))
}

pub fn extract_hierarchy_from_value(value: &Value) -> HashMap<String, Instance> {
    let mut top_level_instances = HashMap::new();
    let symbols = AstSymbols::new(value);

    if let Some(members) = value
        .get("design")
        .and_then(|v| v.get("members").and_then(|v| v.as_array()))
    {
        for member in members {
            let member = symbols.resolve(member);
            if let Some(kind) = member.get("kind") {
                if kind == "Instance" {
                    if let Some((mut inst, value)) =
                        descend_into_instance(member, "".to_string(), &symbols)
                    {
                        extract_hierarchy_from_value_helper(
                            &mut inst,
                            value,
                            "".to_string(),
                            &symbols,
                        );
                        top_level_instances.insert(inst.def_name.clone(), inst);
                    }
                }
            }
        }
    }

    top_level_instances
}

fn extract_hierarchy_from_value_helper(
    top: &mut Instance,
    value: &Value,
    hier_prefix: String,
    symbols: &AstSymbols,
) {
    let symbol_table = create_symbol_table(value);
    if let Some(members) = value.get("members").and_then(|v| v.as_array()) {
        for member in members {
            let member = symbols.resolve(member);
            if let Some(kind) = member.get("kind") {
                if kind == "Instance" {
                    if let Some((mut inst, value)) =
                        descend_into_instance(member, hier_prefix.clone(), symbols)
                    {
                        extract_hierarchy_from_value_helper(
                            &mut inst,
                            value,
                            "".to_string(),
                            symbols,
                        );
                        top.contents.push(Rc::new(RefCell::new(inst)));
                    }
                } else if kind == "UninstantiatedDef" {
                    if let Some(inst_name) = member.get("name").and_then(|v| v.as_str()) {
                        if let Some(def_name) =
                            member.get("definitionName").and_then(|v| v.as_str())
                        {
                            top.contents.push(Rc::new(RefCell::new(Instance {
                                def_name: def_name.to_string(),
                                inst_name: inst_name.to_string(),
                                hier_prefix: hier_prefix.clone(),
                                contents: Vec::new(),
                            })));
                        }
                    }
                } else if kind == "GenerateBlock" {
                    if let Some((hier_prefix, value)) =
                        descend_into_generate_block(member, hier_prefix.clone(), &symbol_table)
                    {
                        extract_hierarchy_from_value_helper(
                            top,
                            value,
                            hier_prefix.clone(),
                            symbols,
                        );
                    }
                } else if kind == "GenerateBlockArray" {
                    if let Some(elements) = descend_into_generate_block_array(
                        member,
                        hier_prefix.clone(),
                        &symbol_table,
                    ) {
                        for (hier_prefix, element) in elements {
                            extract_hierarchy_from_value_helper(
                                top,
                                element,
                                hier_prefix.clone(),
                                symbols,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn descend_into_instance<'a>(
    value: &'a Value,
    hier_prefix: String,
    symbols: &AstSymbols<'a>,
) -> Option<(Instance, &'a Value)> {
    if let Some(inst_name) = value.get("name").and_then(|v| v.as_str()) {
        if inst_name.is_empty() {
            return None;
        }
        if let Some(body) = value.get("body").map(|body| symbols.resolve(body)) {
            if let Some(def_name) = body.get("name").and_then(|v| v.as_str()) {
                if def_name.is_empty() {
                    return None;
                }
                return Some((
                    Instance {
                        def_name: def_name.to_string(),
                        inst_name: inst_name.to_string(),
                        hier_prefix,
                        contents: Vec::new(),
                    },
                    body,
                ));
            }
        }
    }

    None
}

/// Resolves Slang AST links. Slang v10 and newer serialize a node only once and
/// represent subsequent references as strings containing the node address and
/// name (for example, `"123456 C"`). Older releases serialize those nodes
/// inline, so accepting both forms keeps this crate compatible across versions.
struct AstSymbols<'a> {
    /// Every serialized AST object that can be the target of an address link.
    by_address: HashMap<u64, &'a Value>,
}

impl<'a> AstSymbols<'a> {
    /// Indexes the complete JSON document so links can cross AST scopes.
    fn new(root: &'a Value) -> Self {
        let mut by_address = HashMap::new();
        Self::index(root, &mut by_address);
        Self { by_address }
    }

    /// Recursively records each object with an `addr` field.
    fn index(value: &'a Value, by_address: &mut HashMap<u64, &'a Value>) {
        match value {
            Value::Object(object) => {
                if let Some(address) = object.get("addr").and_then(Value::as_u64) {
                    by_address.insert(address, value);
                }
                for child in object.values() {
                    Self::index(child, by_address);
                }
            }
            Value::Array(array) => {
                for child in array {
                    Self::index(child, by_address);
                }
            }
            _ => {}
        }
    }

    /// Follows a v10+ string link to its serialized object.
    ///
    /// Inline objects from older Slang releases are returned unchanged. An
    /// unknown or malformed link is also left untouched, allowing the caller's
    /// existing shape checks to reject it naturally.
    fn resolve(&self, value: &'a Value) -> &'a Value {
        value
            .as_str()
            .and_then(|link| link.split_whitespace().next())
            .and_then(|address| address.parse::<u64>().ok())
            .and_then(|address| self.by_address.get(&address).copied())
            .unwrap_or(value)
    }
}

fn descend_into_generate_block<'a>(
    value: &'a Value,
    hier_prefix: String,
    symbol_table: &HashSet<String>,
) -> Option<(String, &'a Value)> {
    if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
        if let Some(index) = value.get("constructIndex").and_then(|v| v.as_i64()) {
            if let Some(uninstantiated) = value.get("isUninstantiated").and_then(|v| v.as_bool()) {
                if uninstantiated {
                    return None;
                }
                let genblk_name = if name.is_empty() {
                    get_default_genblk_name(index as usize, symbol_table)
                } else {
                    name.to_string()
                };
                let hier_prefix = format!("{hier_prefix}.{genblk_name}");
                return Some((hier_prefix, value));
            }
        }
    }

    None
}

fn descend_into_generate_block_array<'a>(
    value: &'a Value,
    hier_prefix: String,
    symbol_table: &HashSet<String>,
) -> Option<Vec<(String, &'a Value)>> {
    if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
        if let Some(index) = value.get("constructIndex").and_then(|v| v.as_i64()) {
            if let Some(members) = value.get("members").and_then(|v| v.as_array()) {
                let genblk_name = if name.is_empty() {
                    get_default_genblk_name(index as usize, symbol_table)
                } else {
                    name.to_string()
                };
                let hier_prefix = format!("{hier_prefix}.{genblk_name}");

                let mut elements = Vec::new();
                for member in members {
                    if let Some(kind) = member.get("kind") {
                        if kind == "GenerateBlock" {
                            if let Some(subidx) =
                                member.get("constructIndex").and_then(|v| v.as_i64())
                            {
                                elements.push((format!("{hier_prefix}[{subidx}]"), member));
                            }
                        }
                    }
                }
                return Some(elements);
            }
        }
    }

    None
}

fn create_symbol_table(value: &Value) -> HashSet<String> {
    let mut table = HashSet::new();
    if let Some(members) = value.get("members").and_then(|v| v.as_array()) {
        for member in members {
            if let Some(name) = member.get("name") {
                if !name.as_str().unwrap().is_empty() {
                    table.insert(name.as_str().unwrap().to_string());
                }
            }
        }
    }
    table
}

fn get_default_genblk_name(index: usize, symbol_table: &HashSet<String>) -> String {
    let mut prefix = "genblk".to_string();
    while symbol_table.contains(&format!("{prefix}{index}")) {
        prefix = format!("{prefix}0");
    }
    format!("{prefix}{index}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolves_repeated_instance_body_links_from_slang_v11() {
        let ast = json!({
            "design": {
                "members": [{
                    "name": "top",
                    "kind": "Instance",
                    "body": {
                        "name": "top",
                        "kind": "InstanceBody",
                        "addr": 1,
                        "members": [
                            {
                                "name": "first",
                                "kind": "Instance",
                                "body": { "name": "child", "kind": "InstanceBody", "addr": 2 }
                            },
                            { "name": "second", "kind": "Instance", "body": "2 child" }
                        ]
                    }
                }]
            }
        });

        let hierarchy = extract_hierarchy_from_value(&ast);
        let children = &hierarchy["top"].contents;
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].borrow().inst_name, "first");
        assert_eq!(children[1].borrow().inst_name, "second");
        assert_eq!(children[1].borrow().def_name, "child");
    }
}
