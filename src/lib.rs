// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;
use std::io::Write;
use std::process::Command;
use std::{collections::HashMap, fs};

mod extract;
pub use extract::{extract_ports, Port, PortDir};

pub fn run_slang(
    verilog: &str,
    ignore_unknown_modules: bool,
    parameters: &HashMap<String, String>,
) -> Result<Value, Box<dyn std::error::Error>> {
    // Write input to tmp_sv.
    let mut tmp_sv = tempfile::NamedTempFile::new()?;
    tmp_sv.write_all(verilog.as_bytes())?;

    // Run the slang binary, dumping JSON to tmp_json.
    let slang_path = env!("SLANG_PATH");
    let tmp_json = tempfile::NamedTempFile::new()?;
    let mut args = vec!["--ast-json", tmp_json.path().to_str().unwrap()];
    if ignore_unknown_modules {
        args.push("--ignore-unknown-modules");
    }
    let param_args: Vec<String> = parameters
        .iter()
        .map(|(name, value)| format!("{}={}", name, value))
        .collect();
    for param_arg in param_args.iter() {
        args.push("-G");
        args.push(param_arg.as_str());
    }
    args.push(tmp_sv.path().to_str().unwrap());
    let output = Command::new(slang_path).args(args).output()?;

    if !output.status.success() {
        return Err(format!("slang command failed with status: {}", output.status).into());
    }

    // Read and parse the JSON output
    let json_data = fs::read_to_string(tmp_json)?;
    let json_value: Value = serde_json::from_str(&json_data)?;

    Ok(json_value)
}
