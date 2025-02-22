// SPDX-License-Identifier: Apache-2.0

//! Test that the current package version reflected in Cargo.toml is more than
//! the released version -- this help make sure we bump the version
//! appropriately after a release is performed.
//!
//! This is done for all released crates in the workspace.
// from https://github.com/xlsynth/xlsynth-crate/blob/main/test-helpers/tests/version_test.rs

use curl::easy::Easy;

const USER_AGENT: &str = "slang_rs_crate_unit_test";

fn get_workspace_root() -> std::path::PathBuf {
    let workspace_dir = cargo_metadata::MetadataCommand::new()
        .exec()
        .unwrap()
        .workspace_root;
    workspace_dir.into()
}

/// Fetches the latest version of a crate named `crate_name` from crates.io.
fn fetch_latest_version(crate_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
    let mut data = Vec::new();
    let mut easy = Easy::new();
    easy.url(&url)?;
    easy.useragent(USER_AGENT)?;
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|new_data| {
            data.extend_from_slice(new_data);
            Ok(new_data.len())
        })?;
        transfer.perform()?;
    }

    let response: serde_json::Value = serde_json::from_slice(&data)?;
    log::debug!("Response: {:?}", response);
    let newest_version = response["crate"]["newest_version"].as_str();
    let latest_version = newest_version
        .ok_or(format!(
            "Failed to parse latest version: {:?}",
            newest_version
        ))?
        .to_string();
    Ok(latest_version)
}

/// Fetches the local version of a package given the path to a `Cargo.toml`
/// file.
fn fetch_local_version(dirpath: &std::path::PathBuf) -> Result<String, Box<dyn std::error::Error>> {
    let cargo_toml = std::fs::read_to_string(dirpath.join("Cargo.toml"))?;
    let cargo_toml: toml::Value = toml::from_str(&cargo_toml)?;
    let version = cargo_toml["package"]["version"]
        .as_str()
        .ok_or(format!(
            "Failed to parse local version: {}",
            cargo_toml["package"]["version"]
        ))?
        .to_string();
    Ok(version)
}

fn validate_local_version_gt_released(
    crate_name: &str,
    workspace_path: &std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let latest_version = fetch_latest_version(crate_name)?;
    let local_version = fetch_local_version(workspace_path)?;

    let local_semver = semver::Version::parse(&local_version)?;
    let latest_semver = semver::Version::parse(&latest_version)?;

    log::info!(
        "crate: {} local_version: {} released_version: {}",
        crate_name,
        local_version,
        latest_version
    );

    if local_semver <= latest_semver {
        // Technically we're abusing io::Error a bit here just to avoid creating a whole
        // new error type.
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Local version {} is not greater than the latest version {}",
                local_version, latest_version
            ),
        )))
    } else {
        Ok(())
    }
}

#[test]
fn test_slang_rs_crate_version() {
    let _ = env_logger::builder().is_test(true).try_init();
    validate_local_version_gt_released("slang_rs", &get_workspace_root()).unwrap();
}
