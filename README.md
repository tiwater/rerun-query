# Rerun Query

Python SDK to query and extract data from [Rerun](https://rerun.io) files.

## Rerun Compatibility

This project is only compatible with Rerun 0.18. Please make sure the source rrd file is created with this version of Rerun SDK [[Rust](https://docs.rs/rerun/latest/rerun/)].

> The Blueprint of Rerun data (controls the layout of the viewer) is ignored for now, while the entity_paths were still included in the return value of `list_entity_paths()`.
>
> Please open an issue if you need this feature or anything else.

## Install

This project depends on `numpy`, so please make sure install it with this package together.

```bash
pip install numpy rerun-query
```

The retrieved data is in numpy arrays.

## Usage

Use this package:

```py
import requery

data = requery.query_entities(file_path, "")
for data_row in data:
    print(f"Entity Path: {data_row.entity_path}")
    for timeline_key, times in data_row.timelines.items():
        print(f"Timeline({timeline_key}) - {times}")
    data_object = data_row.data
    for index, data in enumerate(data_object[:10]):
        print(f"- {index + 1} {data}")
```

## Example

You can find running example and sample data file in [examples](https://github.com/tiwater/rerun-query/tree/main/examples) folder.

To run the example Python code:

```bash
cd ./examples
python3 -m venv .venv
source .venv/bin/activate
python3 main.py
```

The output includes entity paths, meta data, and tensor data in numpy arrays.

To view the logs in detail, run the program as:

```bash
RUST_LOG=debug python3 main.py
```

If the program crashes unexpectedly, try to diagnose with:

```bash
RUST_BACKTRACE=1 python3 main.py
```
