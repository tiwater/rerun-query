# Contributing

## Development

Maturin is specifically designed to bridge Rust and Python, enabling you to easily create Python bindings for Rust code using PyO3 or rust-cpython. Compared to Cargo, Maturin handles PyO3 more effectively.

```bash
brew install maturin
```

or

```bash
pip install maturin
```

Create a virtual environment and then build the project:

```bash
python3 -m venv .venv
maturin develop
```

Run example code on develop build:

```
cd examples
source ../.venv/bin/activate
RUST_LOG=debug python3 main.py
```

## Publish

```bash
maturin build
maturin publish --username __token__ --password <api-token>
```

The API token can be fetched at https://pypi.org/manage/account/token/ .
