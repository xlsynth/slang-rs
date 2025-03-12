# slang-rs

Parse SystemVerilog with Slang using a Rust API.

Note: the API is currently under development and is subject to frequent changes.

## Installation

Install Rust if you don't have it already:

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Also install cmake if it is not already installed:

Ubuntu:
```shell
sudo apt-get install cmake
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
