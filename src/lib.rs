// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;
use std::fs;
use std::io::Write;
use std::process::Command;

pub fn run_slang(input: &str) -> Result<Value, Box<dyn std::error::Error>> {
    // Write input to tmp_sv.
    let mut tmp_sv = tempfile::NamedTempFile::new()?;
    tmp_sv.write_all(input.as_bytes())?;

    // Run the slang binary, dumping JSON to tmp_json.
    let slang_path = env!("SLANG_PATH");
    let tmp_json = tempfile::NamedTempFile::new()?;
    let output = Command::new(slang_path)
        .args([
            "--ast-json",
            tmp_json.path().to_str().unwrap(),
            tmp_sv.path().to_str().unwrap(),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!("slang command failed with status: {}", output.status).into());
    }

    // Read and parse the JSON output
    let json_data = fs::read_to_string(tmp_json)?;
    let json_value: Value = serde_json::from_str(&json_data)?;

    Ok(json_value)
}
