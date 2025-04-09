// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;
use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub struct Instance {
    pub def_name: String,
    pub inst_name: String,
    pub contents: Vec<Rc<RefCell<Instance>>>,
}

pub fn extract_hierarchy(cfg: &crate::SlangConfig) -> Result<Instance, Box<dyn Error>> {
    extract_hierarchy_from_value(&crate::run_slang(cfg)?)
}

pub fn extract_hierarchy_from_value(value: &Value) -> Result<Instance, Box<dyn Error>> {
    if let Some(members) = value
        .get("design")
        .and_then(|v| v.get("members").and_then(|v| v.as_array()))
    {
        for member in members {
            if let Some((mut inst, value)) = descend_into_instance(member) {
                extract_hierarchy_from_value_helper(&mut inst, value);
                return Ok(inst);
            }
        }
    }

    Err("Top-level module not found".into())
}

fn extract_hierarchy_from_value_helper(top: &mut Instance, value: &Value) {
    if let Some(members) = value.get("members").and_then(|v| v.as_array()) {
        for member in members {
            if let Some((mut inst, value)) = descend_into_instance(member) {
                extract_hierarchy_from_value_helper(&mut inst, value);
                top.contents.push(Rc::new(RefCell::new(inst)));
            }
        }
    }
}

fn descend_into_instance(value: &Value) -> Option<(Instance, &Value)> {
    if let Some(kind) = value.get("kind") {
        if kind == "Instance" {
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
                                contents: Vec::new(),
                            },
                            body,
                        ));
                    }
                }
            }
        }
    }
    None
}
