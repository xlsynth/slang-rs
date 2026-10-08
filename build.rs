// SPDX-License-Identifier: Apache-2.0

#[path = "build/native.rs"]
mod native;

fn main() {
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build/native.rs");
    println!("cargo:rerun-if-changed=cmake/CMakeLists.txt");
    println!("cargo:rerun-if-changed=src/native.rs");
    println!("cargo:rerun-if-changed=cpp/native.cpp");
    println!("cargo:rerun-if-changed=cpp/native.h");

    // docs.rs only type-checks the Rust-facing API; it does not link or run it.
    if std::env::var_os("DOCS_RS").is_some() {
        return;
    }

    let prefix = native::prefix().unwrap_or_else(|error| {
        panic!(
            "Failed to prepare the native Slang library: {error}\n\
             Build and install a compatible prefix with tools/native.py, or unset\n\
             SLANG_RS_NATIVE_DIR to build from the pinned sources. Source builds\n\
             require CMake >= 3.20, Python 3, a C++20 compiler\n\
             (GCC >= 11, Clang >= 17, or Xcode >= 16), and Make or Ninja."
        )
    });
    let mut bridge = cxx_build::bridge("src/native.rs");
    bridge
        .file("cpp/native.cpp")
        .include(".")
        .include(prefix.join("include"))
        .std("c++20");
    for define in native::DEFINES {
        bridge.define(define, None);
    }
    bridge.compile("slang-rs-bridge");
    native::link(&prefix);
}
