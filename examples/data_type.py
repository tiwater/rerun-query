import requery  # type: ignore[import]


def main():
    # Define the file path and entity path you want to query
    # file_path = "./data/robot_action_gripper.rrd"
    file_path = "./data/robot_action_mixed.rrd"
    # file_path = "./data/action_r1_h_not_np.rrd"
    # file_path = "./data/action_r1_g.rrd"

    # Call the query_data_entities function with filter "scalar"
    try:
        data = requery.query_data_entities(file_path, "tensor", "")
        for data_row in data:
            print(f"Entity Path: {data_row.entity_path}")
            for timeline_key, times in data_row.timelines.items():
                print(f"Timeline({timeline_key}) - {times[:10]}")
            data_object = data_row.data
            print(
                f"Only showing at most the first 10 items out of {len(data_object)} total data columns."
            )
            for index, data in enumerate(data_object[:10]):
                print(f"- {index + 1} {data}")
    except Exception as err:
        print(f"Error: {err}")


if __name__ == "__main__":
    main()
