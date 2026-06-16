# slang-rs

Parse SystemVerilog with Slang using a Rust API.

Note: the API is currently under development and is subject to frequent changes.

## Prerequisite

First install the Slang parser. We recommend building it from source.

```shell
curl --fail --location --proto '=https' --tlsv1.2 -o v6.0.tar.gz \
  "https://github.com/MikePopoloski/slang/archive/refs/tags/v6.0.tar.gz"
echo "c4c43f4ef7e2dcaca15541442f9e75ce8749eb0a16224b7c1a6a3c42ae468179  v6.0.tar.gz" | sha256sum -c -
tar xzvf v6.0.tar.gz
```

```shell
cd slang-6.0
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
