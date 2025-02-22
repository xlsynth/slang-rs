# slang-rs

Parse SystemVerilog with Slang using a Rust API.

Note: the API is currently under development and is subject to frequent changes.

## Prerequisite

First install the Slang parser. We recommend building it from source.

```shell
curl -LO "https://github.com/MikePopoloski/slang/archive/refs/tags/v6.0.tar.gz"
```

```shell
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

Install Rust if you don't have it already:

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

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

If you haven't already installed `pre-commit`, you can do so with:

```shell
pip install pre-commit
```

Then install the pre-commit hooks for this repository with:

```shell
pre-commit install
```

The pre-commit hooks will run automatically when you attempt to commit code. You can also run pre-commit checks on-demand with:

```shell
pre-commit run
```
