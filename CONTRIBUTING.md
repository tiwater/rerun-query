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

We use a GitHub Action called "Semantic Release" to manage our publishing process. This approach ensures consistent versioning and automated releases.

Developers can commit and push changes to the repository as needed. These actions do not automatically trigger a new release. When it's time to publish a new version, go to the GitHub Actions page for this repository, locate the "Semantic Release" workflow, and run it

## Best Practices

- Use conventional commit messages to help with automatic versioning.
- Only trigger the "Semantic Release" workflow when you're ready to publish a new version.
- Review the generated release notes before finalizing the release.

By following this process, we maintain control over when releases happen while benefiting from the automation provided by Semantic Release.
