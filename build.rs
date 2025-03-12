// build.rs
use std::{
	env,
	fs,
	path::{Path, PathBuf},
	process::Command,
};

fn main() {
	// Instruct Cargo to re-run this script if build.rs itself changes.
	println!("cargo:rerun-if-changed=build.rs");

	// Define the desired version and tarball URL for Slang.
	const SLANG_VERSION: &str = "v6.0";
	const SLANG_TARBALL_URL: &str =
		"https://github.com/MikePopoloski/slang/archive/refs/tags/v6.0.tar.gz";

	// Get the OUT_DIR to download and extract the source.
	let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
	// We will extract the Slang source into OUT_DIR/slang-src.
	let slang_src_dir = out_dir.join("slang-src");

	// If the Slang source directory does not exist, download and extract it.
	if !slang_src_dir.exists() {
		println!(
			"Downloading Slang {} from {}",
			SLANG_VERSION, SLANG_TARBALL_URL
		);
		// Define the path where the tarball will be saved.
		let tarball_path = out_dir.join("slang.tar.gz");

		// Download the tarball using the "curl" command.
		let status = Command::new("curl")
			.args(&["-L", "-o"])
			.arg(tarball_path.to_str().unwrap())
			.arg(SLANG_TARBALL_URL)
			.status()
			.expect("Failed to execute curl to download Slang tarball");
		assert!(
			status.success(),
			"curl failed to download Slang tarball: {:?}",
			status
		);

		// Extract the tarball using the "tar" command.
		// This will extract the archive into OUT_DIR; typically the directory
		// created will be "slang-6.0".
		let status = Command::new("tar")
			.args(&[
				"-xzf",
				tarball_path.to_str().unwrap(),
				"-C",
				out_dir.to_str().unwrap(),
			])
			.status()
			.expect("Failed to execute tar command to extract Slang tarball");
		assert!(
			status.success(),
			"tar command failed to extract Slang tarball: {:?}",
			status
		);

		// The tarball typically extracts to a folder named "slang-6.0".
		// Rename that folder to "slang-src"
		let extracted_dir = out_dir.join("slang-6.0");
		fs::rename(&extracted_dir, &slang_src_dir)
			.expect("Failed to rename extracted Slang folder");
	} else {
		println!("Using cached Slang source at {:?}", slang_src_dir);
	}

	// Build Slang using the cmake crate.
	// You can pass extra definitions as needed (for example, to disable tests).
	let dst: PathBuf = cmake::Config::new(&slang_src_dir)
		.define("BUILD_TESTS", "OFF")
		// Add additional .define(...) calls if you need to customize the build.
		.build();

	// In this example, we assume that the built binary ends up under ${dst}/bin/slang.
	let slang_bin = dst.join("bin").join("slang");

	// Print a message (a Cargo "directive") so that later your Rust code
	// can retrieve the binary path from the SLANG_PATH environment variable.
	println!("cargo:rustc-env=SLANG_PATH={}", slang_bin.display());

	// Optionally, print a warning if desired:
	println!("cargo:warning=Using built Slang binary at: {}", slang_bin.display());
}