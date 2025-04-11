// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;
use std::collections::{hash_map::Entry, HashMap};
use std::error::Error;
use std::str::FromStr;

mod type_extract;
pub use type_extract::{parse_type_definition, Field, Range, Type, Variant};

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

impl FromStr for PortDir {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "In" => Ok(PortDir::Input),
            "Out" => Ok(PortDir::Output),
            "InOut" => Ok(PortDir::InOut),
            _ => Err(format!("Unsupported I/O direction: {}", s)),
        }
    }
}

pub fn extract_ports(
    cfg: &crate::SlangConfig,
    skip_unsupported: bool,
) -> HashMap<String, Vec<Port>> {
    let result = crate::run_slang(cfg).unwrap();
    extract_ports_from_value(&result, skip_unsupported)
}

pub fn extract_ports_from_value(
    value: &Value,
    skip_unsupported: bool,
) -> HashMap<String, Vec<Port>> {
    let mut ports_map = HashMap::new();

    if let Some(members) = value["design"]["members"].as_array() {
        for member in members {
            if member["kind"] == "Instance" {
                let module_name = member["name"].as_str().unwrap();
                if module_name.is_empty() {
                    continue;
                }
                let mut ports = Vec::new();
                if let Some(instance_body_members) = member["body"]["members"].as_array() {
                    for instance_member in instance_body_members {
                        let kind = instance_member["kind"].as_str().unwrap();
                        match kind {
                            "Port" => {
                                let port_name = instance_member["name"].as_str().unwrap();
                                let direction = instance_member["direction"].as_str().unwrap();
                                let type_str = instance_member["type"].as_str().unwrap();
                                if type_str == "<error>" {
                                    if skip_unsupported {
                                        continue;
                                    } else {
                                        panic!("Found \"<error>\" type in Slang JSON output.");
                                    }
                                }
                                let ty = match parse_type_definition(type_str) {
                                    Ok(ty) => ty,
                                    Err(e) => {
                                        if !skip_unsupported {
                                            panic!("{}", e);
                                        } else {
                                            continue;
                                        }
                                    }
                                };
                                ports.push(Port {
                                    dir: PortDir::from_str(direction).unwrap(),
                                    name: port_name.to_string(),
                                    ty,
                                });
                            }
                            "InterfacePort" => {
                                if !skip_unsupported {
                                    panic!("Interface ports are not currently supported.")
                                }
                            }
                            _ => continue,
                        }
                    }
                }
                match ports_map.entry(module_name.to_string()) {
                    Entry::Vacant(entry) => {
                        entry.insert(ports);
                    }
                    Entry::Occupied(entry) => {
                        panic!("Duplicate definition of module: {}", entry.key());
                    }
                }
            }
        }
    }

    ports_map
}

pub fn extract_modules_from_value(value: &Value) -> Result<Vec<String>, Box<dyn Error>> {
    let definitions = value
        .get("definitions")
        .and_then(|v| v.as_array())
        .ok_or("JSON parsing failed")?;

    let mut modules = Vec::new();

    for definition in definitions {
        if definition.get("kind").and_then(|v| v.as_str()) == Some("Definition")
            && definition.get("definitionKind").and_then(|v| v.as_str()) == Some("Module")
        {
            if let Some(name) = definition.get("name").and_then(|v| v.as_str()) {
                modules.push(name.to_string());
            }
        }
    }

    Ok(modules)
}

pub fn extract_modules(cfg: &crate::SlangConfig) -> Result<Vec<String>, Box<dyn Error>> {
    let result = crate::run_slang(cfg)?;
    extract_modules_from_value(&result)
}
