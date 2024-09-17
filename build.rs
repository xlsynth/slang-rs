// SPDX-License-Identifier: Apache-2.0

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use flate2::read::GzDecoder;
use reqwest::blocking::get;
use tar::Archive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let dst = PathBuf::from(out_dir);

    // Download the slang 6.0 release.
    let response = get("https://github.com/MikePopoloski/slang/archive/refs/tags/v6.0.tar.gz")?;
    let decoder = GzDecoder::new(response);
    let mut archive = Archive::new(decoder);

    // Extract it in the OUT_DIR.
    archive.unpack(&dst)?;

    // Run CMake to configure and build slang.
    let build_dir = dst.join("slang-6.0");
    Command::new("cmake")
        .current_dir(&build_dir)
        .args(["-B", "build"])
        .status()?;
    Command::new("cmake")
        .current_dir(&build_dir)
        .args(["--build", "build", "-j8"])
        .status()?;

    // Copy the slang binary to the target directory.
    let binary_path = build_dir.join("build/bin/slang");
    let slang_path = dst.join("slang");
    fs::copy(binary_path, &slang_path)?;

    // Delete build_dir and its contents.
    fs::remove_dir_all(&build_dir)?;

    // Let the package know where the slang binary is.
    println!(
        "cargo:rustc-env=SLANG_PATH={}",
        slang_path.to_str().unwrap()
    );

    // Rerun this build script if it has changed.
    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}
