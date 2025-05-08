// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;
use std::collections::{hash_map::Entry, HashMap};
use std::error::Error;
use std::hash::Hash;
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

struct MemberIter<'a> {
    members: Option<&'a Vec<Value>>,
    kinds: &'a [&'a str],
    current: usize,
}

impl<'a> MemberIter<'a> {
    fn new(value: &'a Value, kinds: &'a [&'a str]) -> Self {
        Self {
            members: value["members"].as_array(),
            kinds,
            current: 0,
        }
    }
}

impl<'a> Iterator for MemberIter<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(members) = self.members {
            while self.current < members.len() {
                let member = &members[self.current];
                let kind = member["kind"].as_str().unwrap();
                self.current += 1;
                if self.kinds.contains(&kind) {
                    return Some(member);
                }
            }
            None
        } else {
            None
        }
    }
}

fn parse_type_definition_no_error(type_str: &str) -> Result<Type, Box<dyn Error>> {
    if type_str == "<error>" {
        Err("Found \"<error>\" type in Slang JSON output.")?;
    }
    parse_type_definition(type_str)
}

fn insert_to_vacant<K, V>(map: &mut HashMap<K, V>, key: K, value: V) -> Result<(), String>
where
    K: Eq,
    K: Hash,
{
    match map.entry(key) {
        Entry::Vacant(entry) => {
            entry.insert(value);
            Ok(())
        }
        Entry::Occupied(_) => Err(String::from("Duplicate insertion")),
    }
}

pub fn extract_ports_from_value(
    value: &Value,
    skip_unsupported: bool,
) -> HashMap<String, Vec<Port>> {
    let mut ports_map = HashMap::new();

    for member in MemberIter::new(&value["design"], &["Instance"]) {
        let module_name = member["name"].as_str().unwrap();
        if module_name.is_empty() {
            continue;
        }
        let mut ports = Vec::new();
        for instance_member in MemberIter::new(&member["body"], &["Port", "InterfacePort"]) {
            let kind = instance_member["kind"].as_str().unwrap();
            match kind {
                "Port" => {
                    let port_name = instance_member["name"].as_str().unwrap();
                    let direction = instance_member["direction"].as_str().unwrap();
                    let type_str = instance_member["type"].as_str().unwrap();
                    match parse_type_definition_no_error(type_str) {
                        Ok(ty) => ports.push(Port {
                            dir: PortDir::from_str(direction).unwrap(),
                            name: port_name.to_string(),
                            ty,
                        }),
                        Err(e) => {
                            if skip_unsupported {
                                continue;
                            } else {
                                panic!("{}", e);
                            }
                        }
                    }
                }
                "InterfacePort" => {
                    if !skip_unsupported {
                        panic!("Interface ports are not currently supported.")
                    }
                }
                _ => continue,
            }
        }
        insert_to_vacant(&mut ports_map, module_name.to_string(), ports)
            .expect(format!("Duplicate definition of module: {}", module_name).as_str());
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
