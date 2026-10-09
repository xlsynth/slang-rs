// SPDX-License-Identifier: Apache-2.0

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFINES: &[&str] = &[
    "SLANG_STATIC_DEFINE",
    "SLANG_USE_THREADS",
    "SLANG_BOOST_SINGLE_HEADER",
];

/// Select only the upstream native libraries. The bridge is always built by
/// this crate against the selected headers and Rust/cxx version.
pub fn prefix() -> Result<PathBuf, Box<dyn Error>> {
    println!("cargo:rerun-if-env-changed=SLANG_RS_NATIVE_DIR");
    match env::var_os("SLANG_RS_NATIVE_DIR") {
        Some(path) => installed(&PathBuf::from(path)),
        None => build(),
    }
}

/// Emit these after compiling the bridge so static dependencies follow it.
pub fn link(prefix: &Path) {
    let include = prefix.join("include");
    let lib = prefix.join("lib");
    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=static=svlang");
    println!("cargo:rustc-link-lib=static=fmt");
    println!("cargo:root={}", prefix.display());
    println!("cargo:include={}", include.display());
    println!("cargo:lib_dir={}", lib.display());
}

fn installed(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let prefix = path.canonicalize().map_err(|error| {
        format!(
            "cannot open SLANG_RS_NATIVE_DIR {}: {error}",
            path.display()
        )
    })?;
    let manifest = prefix.join("share/manifest.txt");
    println!("cargo:rerun-if-changed={}", manifest.display());
    let metadata = fs::read_to_string(&manifest).map_err(|error| {
        format!(
            "cannot read native manifest {}: {error}",
            manifest.display()
        )
    })?;
    let mut fields = HashMap::new();
    for (key, value) in metadata.lines().filter_map(|line| line.split_once('=')) {
        fields.entry(key).or_insert(value);
    }
    let field = |name: &str| {
        fields
            .get(name)
            .copied()
            .ok_or_else(|| format!("{} is missing {name}", manifest.display()))
    };
    let recipe = format!(
        "{:x}",
        Sha256::digest(include_bytes!("../cmake/CMakeLists.txt"))
    );
    if field("native_recipe_sha256")? != recipe {
        return Err(format!(
            "{} was built with a different native recipe (Slang/fmt sources or options)",
            prefix.display()
        )
        .into());
    }
    let target_os = env::var("CARGO_CFG_TARGET_OS")?;
    let system = match target_os.as_str() {
        "macos" => "Darwin",
        "linux" => "Linux",
        _ => return Err(format!("installed native prefixes do not support {target_os}").into()),
    };
    let arch = env::var("CARGO_CFG_TARGET_ARCH")?;
    let installed_arches = field("processor")?;
    let supports_arch = installed_arches.split(';').any(|processor| {
        let processor = processor.to_ascii_lowercase();
        let normalized = match processor.as_str() {
            "arm64" => "aarch64",
            "amd64" => "x86_64",
            other => other,
        };
        normalized == arch
    });
    if field("system")? != system || !supports_arch {
        return Err(format!(
            "{} targets {}/{}, but Cargo is compiling for {system}/{arch}",
            prefix.display(),
            field("system")?,
            installed_arches,
        )
        .into());
    }

    let include = prefix.join("include");
    if !include.is_dir() {
        return Err(format!("incomplete native prefix: missing {}", include.display()).into());
    }
    println!("cargo:rerun-if-changed={}", include.display());
    for relative in ["lib/libsvlang.a", "lib/libfmt.a"] {
        let archive = prefix.join(relative);
        if !archive.is_file() {
            return Err(format!("incomplete native prefix: missing {}", archive.display()).into());
        }
        println!("cargo:rerun-if-changed={}", archive.display());
    }
    Ok(prefix)
}

/// Build the pinned native sources with CMake.
fn build() -> Result<PathBuf, Box<dyn Error>> {
    // A bridge edit may rerun this function, but CMake retains its native objects.
    track_native_environment()?;

    let package = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("missing manifest dir")?);
    let out = PathBuf::from(env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?);
    let native = out.join("native");

    // Dependency pins and compiler/toolchain selections must not reuse a CMake
    // cache configured for different inputs. Pin changes also discard objects
    // on older CMake versions that preserve timestamps when extracting sources.
    // Ordinary Rust edits, launcher changes and build parallelism do not enter
    // this fingerprint and do not discard native objects.
    let input_stamp = out.join("native-inputs");
    let inputs = native_fingerprint()?;
    if fs::read_to_string(&input_stamp).ok().as_deref() != Some(&inputs) && native.exists() {
        // This directory belongs exclusively to this package's Cargo OUT_DIR.
        fs::remove_dir_all(&native)?;
    }
    fs::write(input_stamp, inputs)?;

    let mut config = cmake::Config::new(package.join("cmake"));
    config
        .out_dir(&native)
        // Rust dev/check/test should all use the same optimized native configuration.
        // Cargo still manages distinct OUT_DIRs for distinct target/profile variants.
        .profile("Release")
        .pic(true);

    // Allow verified, already-extracted sources for builds without network access.
    // Clear cached overrides when their environment variables are removed.
    for key in [
        "FETCHCONTENT_SOURCE_DIR_SLANG",
        "FETCHCONTENT_SOURCE_DIR_FMT",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
        if let Some(path) = env::var_os(key).filter(|value| !value.is_empty()) {
            let path = PathBuf::from(path).canonicalize()?;
            println!("cargo:rerun-if-changed={}", path.display());
            config.define(key, path);
        } else {
            config.define(key, "");
        }
    }

    // CMake supports compiler launchers for measuring or caching native builds.
    config.define(
        "CMAKE_CXX_COMPILER_LAUNCHER",
        env::var_os("CMAKE_CXX_COMPILER_LAUNCHER").unwrap_or_default(),
    );
    config.define(
        "Python_EXECUTABLE",
        env::var_os("Python_EXECUTABLE").unwrap_or_default(),
    );

    // Keep CMake's incremental build tree. Reconfiguration is inexpensive and
    // ensures compiler flags / toolchain changes are applied instead of ignored.
    let prefix = config.build();
    Ok(prefix)
}

fn track_native_environment() -> Result<(), Box<dyn Error>> {
    let target = env::var("TARGET")?;
    let target_underscored = target.replace('-', "_");
    let kind = if env::var("HOST")? == target {
        "HOST"
    } else {
        "TARGET"
    };
    // cmake uses cc with cargo_metadata(false), so explicitly track the cc
    // environment as well as the CMake and platform configuration we support.
    for key in [
        "CC",
        "CXX",
        "AR",
        "CFLAGS",
        "CXXFLAGS",
        "ARFLAGS",
        "CMAKE",
        "CMAKE_GENERATOR",
        "CMAKE_TOOLCHAIN_FILE",
        "CMAKE_PREFIX_PATH",
    ] {
        for name in [
            key.to_string(),
            format!("{key}_{target}"),
            format!("{key}_{target_underscored}"),
            format!("{kind}_{key}"),
        ] {
            println!("cargo:rerun-if-env-changed={name}");
            if key == "CMAKE_TOOLCHAIN_FILE" {
                if let Some(path) = env::var_os(&name) {
                    println!("cargo:rerun-if-changed={}", PathBuf::from(path).display());
                }
            }
        }
    }
    for name in [
        "CMAKE_CXX_COMPILER_LAUNCHER",
        "Python_EXECUTABLE",
        "SDKROOT",
        "MACOSX_DEPLOYMENT_TARGET",
        "IPHONEOS_DEPLOYMENT_TARGET",
        "CRATE_CC_NO_DEFAULTS",
        "CXXSTDLIB",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    Ok(())
}

fn native_fingerprint() -> Result<String, Box<dyn Error>> {
    let target = env::var("TARGET")?;
    let target_underscored = target.replace('-', "_");
    let kind = if env::var("HOST")? == target {
        "HOST"
    } else {
        "TARGET"
    };
    let mut hash = Sha256::new();
    hash.update(include_bytes!("../cmake/CMakeLists.txt"));
    for key in ["CC", "CXX", "CMAKE_GENERATOR", "CMAKE_TOOLCHAIN_FILE"] {
        let value = [
            format!("{key}_{target}"),
            format!("{key}_{target_underscored}"),
            format!("{kind}_{key}"),
            key.to_string(),
        ]
        .iter()
        .find_map(env::var_os);
        hash.update(key);
        if let Some(value) = value {
            let bytes = value.as_encoded_bytes();
            hash.update(bytes.len().to_le_bytes());
            hash.update(bytes);
            if key == "CMAKE_TOOLCHAIN_FILE" {
                hash.update(fs::read(PathBuf::from(value))?);
            }
        }
        hash.update([0]);
    }
    for key in [
        "SDKROOT",
        "MACOSX_DEPLOYMENT_TARGET",
        "IPHONEOS_DEPLOYMENT_TARGET",
        "FETCHCONTENT_SOURCE_DIR_SLANG",
        "FETCHCONTENT_SOURCE_DIR_FMT",
    ] {
        hash.update(key);
        if let Some(value) = env::var_os(key) {
            let bytes = value.as_encoded_bytes();
            hash.update(bytes.len().to_le_bytes());
            hash.update(bytes);
        }
        hash.update([0]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
