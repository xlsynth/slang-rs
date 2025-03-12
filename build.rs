use std::{
	env,
	fs,
	path::{PathBuf},
	process::Command,
};

fn main() {
	// Instruct Cargo to re-run this script if build.rs itself changes.
	println!("cargo:rerun-if-changed=build.rs");

	// Define the desired version for Slang.
	const SLANG_VERSION: &str = "6.0";
	// Compute the tarball URL at runtime.
	let slang_tarball_url = format!(
		"https://github.com/MikePopoloski/slang/archive/refs/tags/v{}.tar.gz",
		SLANG_VERSION
	);

	// Get the OUT_DIR to download and extract the source.
	let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
	// We will extract the Slang source into OUT_DIR/slang-src.
	let slang_src_dir = out_dir.join("slang-src");

	// If the Slang source directory does not exist, download and extract it.
	if !slang_src_dir.exists() {
		println!("Downloading Slang {} from {}", SLANG_VERSION, slang_tarball_url);
		// Define the path where the tarball will be saved.
		let tarball_path = out_dir.join(format!("{SLANG_VERSION}.tar.gz"));
		// Download the tarball using the "curl" command.
		let status = Command::new("curl")
			.args(&["-L", "-o"])
			.arg(tarball_path.to_str().unwrap())
			.arg(&slang_tarball_url)
			.status()
			.expect("Failed to execute curl to download Slang tarball");
		assert!(status.success(), "curl failed to download Slang tarball: {:?}", status);
		// Extract the tarball using the "tar" command.
		let status = Command::new("tar")
			.args(&["-xzf", tarball_path.to_str().unwrap(), "-C", out_dir.to_str().unwrap()])
			.status()
			.expect("Failed to execute tar command to extract Slang tarball");
		assert!(status.success(), "tar command failed to extract Slang tarball: {:?}", status);
		// Rename the extracted folder to "slang-src"
		let extracted_dir = out_dir.join(format!("slang-{SLANG_VERSION}"));
		fs::rename(&extracted_dir, &slang_src_dir)
			.expect("Failed to rename extracted Slang folder");
	} else {
		println!("Using cached Slang source at {:?}", slang_src_dir);
	}

	// Build Slang using the cmake crate with cxx17
	let dst: PathBuf = cmake::Config::new(&slang_src_dir)
		.define("BUILD_TESTS", "OFF")        // disable tests as before
		.define("CMAKE_CXX_STANDARD", "17")    // force C++17
		.build();

	// In this example, the built binary is expected under ${dst}/bin/slang.
	let slang_bin = dst.join("bin").join("slang");
	println!("cargo:rustc-env=SLANG_PATH={}", slang_bin.display());
	println!("cargo:warning=Using built Slang binary at: {}", slang_bin.display());
}