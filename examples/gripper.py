import requery  # type: ignore[import]


def main():
    # Define the file path and entity path you want to query
    file_path = "./data/robot_action_gripper.rrd"

    # Call the query_action_entities function
    try:
        data = requery.query_action_entities(file_path, "")
        for data_row in data:
            print(f"Entity Path: {data_row.entity_path}")
            for timeline_key, times in data_row.timelines.items():
                print(f"Timeline({timeline_key}) - {times}")
            data_object = data_row.data
            for index, data in enumerate(data_object[:10]):
                print(f"- {index + 1} {data}")
    except Exception as err:
        print(f"Error: {err}")


if __name__ == "__main__":
    main()
