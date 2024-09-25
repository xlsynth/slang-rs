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

#[derive(Default, Debug, PartialEq)]
pub struct Dims {
    pub packed: Vec<(usize, usize)>,
    pub unpacked: Vec<(usize, usize)>,
}

impl Dims {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug, PartialEq)]
pub struct Port {
    pub dir: PortDir,
    pub name: String,
    pub dims: Dims,
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
                let mut ports = Vec::new();
                if let Some(instance_body_members) = member["body"]["members"].as_array() {
                    for instance_member in instance_body_members {
                        let kind = instance_member["kind"].as_str().unwrap();
                        match kind {
                            "Port" => {
                                let port_name = instance_member["name"].as_str().unwrap();
                                let direction = instance_member["direction"].as_str().unwrap();
                                let type_str = instance_member["type"].as_str().unwrap();
                                match extract_dims(type_str) {
                                    Ok(dims) => {
                                        ports.push(Port {
                                            dir: PortDir::from_str(direction).unwrap(),
                                            name: port_name.to_string(),
                                            dims,
                                        });
                                    }
                                    Err(e) => {
                                        if !skip_unsupported {
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

fn extract_dims(type_str: &str) -> Result<Dims, String> {
    if type_str.contains("struct")
        || type_str.contains("union")
        || type_str.contains("interface")
        || type_str.contains('(')
        || type_str.contains(')')
    {
        return Err(format!("Unsupported type: {}", type_str));
    }

    let mut dims = Dims::new();

    // Split the type string into the base type and dimensions
    let parts: Vec<&str> = type_str.split('$').collect();
    let base_and_packed = parts.first().unwrap_or(&"");
    let unpacked = parts.get(1).unwrap_or(&"");

    // Extract packed dimensions
    let packed_dims: Vec<&str> = base_and_packed.split('[').skip(1).collect();
    for dim in packed_dims {
        if let Some(end_idx) = dim.find(']') {
            let range_str = &dim[..end_idx];
            let mut parts = range_str.split(':');
            if let (Some(msb_str), Some(lsb_str)) = (parts.next(), parts.next()) {
                if let (Ok(msb), Ok(lsb)) = (
                    msb_str.trim().parse::<usize>(),
                    lsb_str.trim().parse::<usize>(),
                ) {
                    dims.packed.push((msb, lsb));
                } else {
                    return Err(format!("Invalid packed dimension: {}", range_str));
                }
            } else {
                return Err(format!("Invalid packed dimension: {}", range_str));
            }
        }
    }

    // Extract unpacked dimensions
    let unpacked_dims: Vec<&str> = unpacked.split('[').skip(1).collect();
    for dim in unpacked_dims {
        if let Some(end_idx) = dim.find(']') {
            let range_str = &dim[..end_idx];
            let mut parts = range_str.split(':');
            if let (Some(msb_str), Some(lsb_str)) = (parts.next(), parts.next()) {
                if let (Ok(msb), Ok(lsb)) = (
                    msb_str.trim().parse::<usize>(),
                    lsb_str.trim().parse::<usize>(),
                ) {
                    dims.unpacked.push((msb, lsb));
                } else {
                    return Err(format!("Invalid unpacked dimension: {}", range_str));
                }
            } else {
                return Err(format!("Invalid unpacked dimension: {}", range_str));
            }
        }
    }

    Ok(dims)
}
