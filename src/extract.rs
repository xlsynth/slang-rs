// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;
use std::collections::{hash_map::Entry, HashMap};
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum PortDir {
    Input,
    Output,
    InOut,
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

#[derive(Debug, PartialEq)]
pub struct Port {
    pub dir: PortDir,
    pub name: String,
    pub msb: usize,
    pub lsb: usize,
}

pub fn extract_ports(
    verilog: &str,
    ignore_unknown_modules: bool,
    parameters: &HashMap<String, String>,
) -> HashMap<String, Vec<Port>> {
    let result = crate::run_slang(verilog, ignore_unknown_modules, parameters).unwrap();
    extract_ports_from_value(&result)
}

fn extract_ports_from_value(value: &Value) -> HashMap<String, Vec<Port>> {
    let mut ports_map = HashMap::new();

    if let Some(members) = value["design"]["members"].as_array() {
        for member in members {
            if member["kind"] == "Instance" {
                let module_name = member["name"].as_str().unwrap();
                let mut ports = Vec::new();
                if let Some(instance_body_members) = member["body"]["members"].as_array() {
                    for instance_member in instance_body_members {
                        let kind = instance_member["kind"].as_str().unwrap();
                        match kind {
                            "Port" => {
                                let port_name = instance_member["name"].as_str().unwrap();
                                let direction = instance_member["direction"].as_str().unwrap();
                                let type_str = instance_member["type"].as_str().unwrap();
                                let (msb, lsb) = extract_msb_lsb(type_str);
                                ports.push(Port {
                                    dir: PortDir::from_str(direction).unwrap(),
                                    name: port_name.to_string(),
                                    msb,
                                    lsb,
                                });
                            }
                            "InterfacePort" => {
                                panic!("Interface ports are not currently supported.")
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

fn extract_msb_lsb(type_str: &str) -> (usize, usize) {
    if type_str.contains('$')
        || type_str.contains("struct")
        || type_str.contains("union")
        || type_str.contains("interface")
        || type_str.contains('(')
        || type_str.contains(')')
        || (type_str.matches('[').count() > 1)
    {
        panic!("Unsupported type: {}", type_str);
    }

    // Extract the bit range from the type string logic[msb:lsb]
    if let Some(start_idx) = type_str.find('[') {
        if let Some(end_idx) = type_str.find(']') {
            let range_str = &type_str[start_idx + 1..end_idx];
            let mut parts = range_str.split(':');
            if let (Some(msb_str), Some(lsb_str)) = (parts.next(), parts.next()) {
                if let (Ok(msb), Ok(lsb)) = (
                    msb_str.trim().parse::<usize>(),
                    lsb_str.trim().parse::<usize>(),
                ) {
                    return (msb, lsb);
                } else {
                    panic!("Invalid type: {}", type_str);
                }
            } else {
                panic!("Invalid type: {}", type_str);
            }
        }
    }

    // If we get to this point, this is a single bit type.
    (0, 0)
}
