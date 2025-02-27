#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0

# from https://github.com/xlsynth/xlsynth-crate/blob/main/check_version_is.py

import sys
import re
import os

if len(sys.argv) < 2:
    print("Usage: check_version_is.py <version>")
    sys.exit(1)

# The argument might be e.g. "v0.0.57" or "0.0.57"
arg_version = sys.argv[1]
print(f"Argument version: {arg_version!r}")

# If you expect the script to always receive e.g. "v0.0.57",
# you can strip the leading 'v' here:
tag_version = arg_version.lstrip('v')
print(f"Tag version:       {tag_version!r}")

# Read the version from Cargo.toml
toml_path = "Cargo.toml"
with open(toml_path, "r", encoding="utf-8") as f:
    cargo_toml = f.read()

print(f"Cargo.toml: {cargo_toml!r}")

# Use a regex to extract the version from a line like: version = "0.0.57"
match = re.search(r'^version\s*=\s*"([^"]+)"', cargo_toml, re.MULTILINE)
if not match:
    print("Error: Could not find a valid `version = \"...\"` line in Cargo.toml.")
    sys.exit(1)

cargo_version = match.group(1)

print(f"Tag version:       {tag_version!r}")
print(f"Cargo.toml version: {cargo_version!r}")

# Compare the extracted version with the tag version
if cargo_version != tag_version:
    print(
        f"Error: version mismatch. "
        f"Tag is {tag_version!r}, but Cargo.toml is {cargo_version!r}."
    )
    sys.exit(1)

print("Success: Tag version matches Cargo.toml version.")
