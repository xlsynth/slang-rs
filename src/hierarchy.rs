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

    if let Some(members) = value
        .get("design")
        .and_then(|v| v.get("members").and_then(|v| v.as_array()))
    {
        for member in members {
            if let Some(kind) = member.get("kind") {
                if kind == "Instance" {
                    if let Some((mut inst, value)) = descend_into_instance(member, "".to_string()) {
                        extract_hierarchy_from_value_helper(&mut inst, value, "".to_string());
                        top_level_instances.insert(inst.def_name.clone(), inst);
                    }
                }
            }
        }
    }

    top_level_instances
}

fn extract_hierarchy_from_value_helper(top: &mut Instance, value: &Value, hier_prefix: String) {
    let symbol_table = create_symbol_table(value);
    if let Some(members) = value.get("members").and_then(|v| v.as_array()) {
        for member in members {
            if let Some(kind) = member.get("kind") {
                if kind == "Instance" {
                    if let Some((mut inst, value)) =
                        descend_into_instance(member, hier_prefix.clone())
                    {
                        extract_hierarchy_from_value_helper(&mut inst, value, "".to_string());
                        top.contents.push(Rc::new(RefCell::new(inst)));
                    }
                } else if kind == "GenerateBlock" {
                    if let Some((hier_prefix, value)) =
                        descend_into_generate_block(member, hier_prefix.clone(), &symbol_table)
                    {
                        extract_hierarchy_from_value_helper(top, value, hier_prefix.clone());
                    }
                } else if kind == "GenerateBlockArray" {
                    if let Some(elements) = descend_into_generate_block_array(
                        member,
                        hier_prefix.clone(),
                        &symbol_table,
                    ) {
                        for (hier_prefix, element) in elements {
                            extract_hierarchy_from_value_helper(top, element, hier_prefix.clone());
                        }
                    }
                }
            }
        }
    }
}

fn descend_into_instance(value: &Value, hier_prefix: String) -> Option<(Instance, &Value)> {
    if let Some(inst_name) = value.get("name").and_then(|v| v.as_str()) {
        if inst_name.is_empty() {
            return None;
        }
        if let Some(body) = value.get("body") {
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
