# slang-rs

Rust interface to the [Slang](https://github.com/MikePopoloski/slang) SystemVerilog
library. Parse and elaborate sources
in-process, then query module ports, parameter types, hierarchy, and packages.

The API is under development and may change between releases.

## Installation

Add `slang-rs` to your `Cargo.toml`:

```toml
[dependencies]
slang-rs = "0.23"
```

Cargo builds Slang and the Rust bindings automatically, downloading and
verifying the Slang and fmt sources as needed.

Building the library requires:

- Rust 1.85.0 or newer.
- CMake 3.20 or newer, Python 3, and Make or Ninja.
- A C++20 compiler: GCC 11+, Clang 17+, or Xcode 16+.

Slang is linked statically, so no separate Slang shared library is needed at runtime.

## Basic usage

Read the ports of a SystemVerilog module:

```rust
use slang_rs::{Compilation, SlangConfig, Source};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = SlangConfig {
        sources: &[Source::text(
            "top.sv",
            "module top(input logic [7:0] data); endmodule",
        )],
        ..Default::default()
    };
    let compilation = Compilation::new(&config)?;
    let ports = compilation.ports()?;
    for port in &ports["top"] {
        println!("{}: {} bits", port.name, port.ty.width()?);
    }
    Ok(())
}
```

Access a port by name with `ports["top"]["data"]`
or by position with `ports["top"][0]`. Iteration follows Slang's port-list order.
Use `Source::file("top.sv")` to read an existing file.

## Packages and types

Package value parameters are available in `Package.parameters` as evaluated
`ConstantValue` values: integers, reals, byte strings, and compound values.
Integers and enum values preserve width and signedness.
Use `TryFrom` and `TryInto` for checked conversions to Rust integer types.
Typedefs are in `Package.types`:

```rust
use slang_rs::{Compilation, SlangConfig, Source};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = SlangConfig {
        sources: &[Source::text("constants.sv", r#"
            package my_pkg;
                localparam int COUNT = 42;
                localparam bit [127:0] WIDE = 128'h1_0000_0000_0000_0000;
                typedef logic [15:0] word_t;
            endpackage
        "#)],
        ..Default::default()
    };
    let packages = Compilation::new(&config)?.packages()?;
    let pkg = &packages["my_pkg"];
    let count = u64::try_from(&pkg.parameters["COUNT"])?;
    let wide: num_bigint::BigInt = (&pkg.parameters["WIDE"]).try_into()?;
    let word_type = pkg.types["word_t"].as_ref().unwrap();
    assert_eq!(count, 42);
    assert_eq!(wide, num_bigint::BigInt::from(1u8) << 64);
    assert_eq!(word_type.width()?, 16);
    Ok(())
}
```

`Type::width()` returns the width in bits, including packed dimensions, and
returns an error for unpacked types. Structs and unions expose whether they are
packed. Packed dimensions expose `left` (MSB) and `right` (LSB) bounds.

Navigate with `hierarchy["top"]["mid"]["child"]` or index children by position,
such as `hierarchy["top"][0]`. Iterate with `for child in &hierarchy["top"]`
or `hierarchy["top"].values()`. Child lookup
uses paths relative to the parent, including array indices and generate scopes;
`Instance.path` is the full hierarchical path.

## Compiler configuration

`SlangConfig` controls parsing, elaboration, and diagnostics:

```rust
use slang_rs::{
    DiagnosticOptions, DiagnosticOverride, DiagnosticSeverity, LanguageVersion,
    SlangConfig, Source, WarningPolicy,
};

let config = SlangConfig {
    sources: &[
        Source::text("settings.sv", "`define DATA_WIDTH 8\n"),
        Source::file("rtl/top.sv"),
        Source::file("rtl/leaf.sv"),
    ],
    tops: &["top"],
    language_version: LanguageVersion::SystemVerilog2023,
    single_unit: true,
    diagnostics: DiagnosticOptions {
        warnings: WarningPolicy::All,
        overrides: &[DiagnosticOverride {
            name: "unused-variable",
            severity: DiagnosticSeverity::Ignored,
        }],
        ..Default::default()
    },
    ..Default::default()
};
```

Sources can be file paths or patterns (`Source::file`) and named in-memory
buffers (`Source::text`). Use `incdirs` for include directories and `tops` to
select top-level modules. Set macros with `defines` and override parameters
with `parameters`.

Sources load in `sources` order. With `single_unit: true`, they share preprocessor
definitions in that order.

Diagnostics are collected without printing. Use `Compilation::diagnostics()`
to inspect warnings; failures carry diagnostics in `Error`. See the
[API documentation](https://docs.rs/slang-rs) for all configuration options.

Keep a `Compilation` to query `ports`, `parameters`, `modules`, `hierarchy`,
or `packages` without recompiling the sources. These queries return
`Result<_, Error>` with structured diagnostics. Their results are owned
Rust data that remains usable after the compilation is dropped. Each compilation
is confined to its creating thread; independent compilations can be created on
different threads.

## Optional precompiled Slang installation

To speed up `slang-rs` builds, you can prebuild Slang and fmt and install them
in a fixed location. Rust and binding changes do not require rebuilding these
libraries; changing the underlying Slang version does. This is especially useful
in CI without a warm cache.

From a `slang-rs` checkout, install for the current user:

```shell
python3 tools/native.py install
```

Run the printed `export SLANG_RS_NATIVE_DIR=...` command, then build your
project normally. Use `unset SLANG_RS_NATIVE_DIR` to return to source builds.

For system installation:

```shell
stage="$(mktemp -d)"
python3 tools/native.py build --output "$stage"
sudo python3 tools/native.py install --from "$stage" \
  --prefix '/opt/slang/<native-version>/<target>' \
  --owner '<owner>' --group '<group>' --file-mode 0644 --dir-mode 0755
```

`slang-rs` GitHub releases provide prebuilt archives for matching GitHub Actions
runners; see the [release workflow](.github/workflows/publish.yml) for supported
platforms. Pin and verify the archive's SHA-256, extract it, and set
`SLANG_RS_NATIVE_DIR` to the extracted directory. The Rust bindings and C++ bridge
build locally and should build quickly.

## Development

When contributing, run the tests and format your changes:

```shell
cargo test --locked
cargo fmt
```

Use `cargo fmt -- --check` to check formatting without modifying files.
