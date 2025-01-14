// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;
use std::fs::{self, write};
use std::process::Command;

mod extract;
pub use extract::{extract_modules, extract_ports, Field, Port, PortDir, Range, Type, Variant};

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
    pub ignore_protected: bool,
    pub timescale: Option<&'a str>,
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
            ignore_protected: true,
            timescale: None,
        }
    }
}

/// Searches the PATH to try and locate the slang binary.
fn find_slang() -> Option<String> {
    which::which("slang")
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

/// Adds options needed to make slang ignore protected envelopes.
fn push_options_to_ignore_protected(args: &mut Vec<&str>) {
    let options = vec![
        "--enable-legacy-protect",
        "-Wno-protected-envelope",
        "-Wno-expected-protect-arg",
        "-Wno-expected-protect-keyword",
        "-Wno-extra-protect-end",
        "-Wno-invalid-encoding-byte",
        "-Wno-invalid-pragma-number",
        "-Wno-invalid-pragma-viewport",
        "-Wno-nested-protect-begin",
        "-Wno-protect-arglist",
        "-Wno-protect-encoding-bytes",
        "-Wno-protected-envelope",
        "-Wno-raw-protect-eof",
        "-Wno-unknown-protect-encoding",
        "-Wno-unknown-protect-keyword",
        "-Wno-unknown-protect-option",
    ];

    for option in options {
        args.push(option);
    }
}

pub fn run_slang(cfg: &SlangConfig) -> Result<Value, Box<dyn std::error::Error>> {
    // Run the slang binary, dumping JSON to tmp_json.

    let slang_path = std::env::var("SLANG_PATH").unwrap_or_else(|_| {
        // fallback solution: try to find slang on our own
        match find_slang() {
            Some(val) => val,
            None => panic!(
                "Please set the SLANG_PATH environment variable to the path of the slang binary."
            ),
        }
    });

    if !std::path::Path::new(&slang_path).exists() {
        return Err(format!("slang binary not found at path: {}", slang_path).into());
    }

    let tmp_json = tempfile::NamedTempFile::new()?;
    let mut args = vec!["--ast-json", tmp_json.path().to_str().unwrap()];

    if cfg.ignore_unknown_modules {
        args.push("--ignore-unknown-modules");
    }

    if cfg.ignore_protected {
        push_options_to_ignore_protected(&mut args);
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

    if let Some(timescale) = cfg.timescale {
        args.push("--timescale");
        args.push(timescale);
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
