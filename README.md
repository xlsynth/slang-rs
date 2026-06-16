# slang-rs

Parse SystemVerilog with Slang using a Rust API.

Note: the API is currently under development and is subject to frequent changes.

## Slang version compatibility

Slang 11.0 is the currently supported release and the version used by this
project's CI and installation instructions.

Older Slang releases may continue to work, but are considered deprecated and
are not guaranteed to remain compatible in future `slang-rs` releases. Slang
7.0 has been tested with the current compatibility layer; other older versions
have not been validated.

## Prerequisite

First install the Slang parser. We recommend building it from source.

```shell
curl --fail --location --proto '=https' --tlsv1.2 -o v11.0.tar.gz \
  "https://github.com/MikePopoloski/slang/archive/refs/tags/v11.0.tar.gz"
echo "50676d5a9adbefb97d266a4b174e6b0513901afd5ac57a6cdfea0a61149c3704  v11.0.tar.gz" | sha256sum -c -
tar xzvf v11.0.tar.gz
```

```shell
cd slang-11.0
```

```shell
cmake -B build
```

```shell
cmake --build build -j8
```

This will take a few minutes. Then set an environment variable to specify the location of the Slang binary:

```shell
export SLANG_PATH=`realpath build/bin/slang`
```

## Installation

Install Rust 1.85.0 or newer using the authenticated installation method for
your platform. CI currently pins Rust 1.85.0, the first release supporting the
Rust 2024 edition.

Then clone this repository:

```shell
git clone https://github.com/xlsynth/slang-rs.git
```

To run the tests:

```shell
cd slang-rs
```

```shell
cargo test
```

## Development

We use [pre-commit](https://pre-commit.com/) as part of our CI pipeline.

The development lock file targets Python 3.12. Create and activate a Python
3.12 virtual environment, then install the hash-locked tools:

```shell
python3.12 -m venv .venv
source .venv/bin/activate
python -m pip install --require-hashes -r requirements-ci.txt
```

Then install the pre-commit hooks for this repository with:

```shell
pre-commit install
```

The pre-commit hooks will run automatically when you attempt to commit code. You can also run pre-commit checks on-demand with:

```shell
pre-commit run
```
