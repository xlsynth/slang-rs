// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;
use std::fs::{self, write};
use std::process::Command;

mod extract;
pub use extract::{extract_ports, Dims, Port, PortDir};

#[derive(Debug)]
pub struct SlangConfig<'a> {
    pub sources: &'a [&'a str],
    pub tops: &'a [&'a str],
    pub incdirs: &'a [&'a str],
    pub defines: &'a [(&'a str, &'a str)],
    pub parameters: &'a [(&'a str, &'a str)],
    pub libfiles: &'a [&'a str],
    pub libdirs: &'a [&'a str],
    pub libexts: &'a [&'a str],
    pub ignore_unknown_modules: bool,
}

impl<'a> Default for SlangConfig<'a> {
    fn default() -> Self {
        SlangConfig {
            sources: &[],
            tops: &[],
            incdirs: &[],
            defines: &[],
            parameters: &[],
            libfiles: &[],
            libdirs: &[],
            libexts: &[],
            ignore_unknown_modules: true,
        }
    }
}

pub fn run_slang(cfg: &SlangConfig) -> Result<Value, Box<dyn std::error::Error>> {
    // Run the slang binary, dumping JSON to tmp_json.
    let slang_path = env!("SLANG_PATH");

    let tmp_json = tempfile::NamedTempFile::new()?;
    let mut args = vec!["--ast-json", tmp_json.path().to_str().unwrap()];

    if cfg.ignore_unknown_modules {
        args.push("--ignore-unknown-modules");
    }

    let param_args: Vec<String> = cfg
        .parameters
        .iter()
        .map(|(name, value)| format!("{}={}", name, value))
        .collect();
    for param_arg in param_args.iter() {
        args.push("-G");
        args.push(param_arg.as_str());
    }

    for top in cfg.tops.iter() {
        args.push("--top");
        args.push(top);
    }

    for incdir in cfg.incdirs.iter() {
        args.push("-I");
        args.push(incdir);
    }

    let define_args: Vec<String> = cfg
        .defines
        .iter()
        .map(|(name, value)| format!("{}={}", name, value))
        .collect();
    for define_arg in define_args.iter() {
        args.push("-D");
        args.push(define_arg.as_str());
    }

    for libfile in cfg.libfiles.iter() {
        args.push("-v");
        args.push(libfile);
    }

    for libdir in cfg.libdirs.iter() {
        args.push("-y");
        args.push(libdir);
    }

    for libext in cfg.libexts.iter() {
        args.push("-Y");
        args.push(libext);
    }

    for source in cfg.sources.iter() {
        args.push(source);
    }

    let output = Command::new(slang_path).args(args).output()?;

    if !output.status.success() {
        return Err(format!(
            "slang command failed with exit code: {}, stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // Read and parse the JSON output
    let json_data = fs::read_to_string(tmp_json)?;
    let json_value: Value = serde_json::from_str(&json_data)?;

    Ok(json_value)
}

pub fn str2tmpfile(s: &str) -> Result<tempfile::NamedTempFile, Box<dyn std::error::Error>> {
    let tmp_file = tempfile::NamedTempFile::new()?;
    write(tmp_file.path(), s)?;
    Ok(tmp_file)
}
