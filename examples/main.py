import requery


def main():
    print("Running Python example...")

    # Define the file path and entity path you want to query
    file_path = "./data/sample-0.18.rrd"

    # Call the list_entity_paths function
    try:
        data = requery.list_entity_paths(file_path)
        print("Query entity paths successful!")
        for index, path in enumerate(data):
            print(f"- Path {index + 1}: {path}")
    except Exception as err:
        print(f"Error: {err}")

    # Call the query_meta_entities function
    meta_entity_path = ""  # You can set as "/meta"
    try:
        print("Query meta chunks")
        data = requery.query_meta_entities(file_path, meta_entity_path)
        for index, row_meta in enumerate(data):
            print(f"- Row {index + 1} {row_meta.entity_path} : {row_meta.text}")
    except Exception as err:
        print(f"Error: {err}")

    # Call the query_action_entities function
    try:
        print("Query action chunks")
        data = requery.query_data_entities(file_path, "", "")
        for data_row in data:
            print(f"Entity Path: {data_row.entity_path}")
            for timeline_key, times in data_row.timelines.items():
                print(f"Timeline({timeline_key}) - {times}")
            data_object = data_row.data
            if len(data_object) > 10:
                print(
                    f"Only showing the first 10 items out of {len(data_object)} total data columns."
                )
            for index, data in enumerate(data_object[:10]):
                print(f"- {index + 1} {data}")
    except Exception as err:
        print(f"Error: {err}")


if __name__ == "__main__":
    main()
