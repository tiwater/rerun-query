use env_logger;
use log::{debug, error};
use re_chunk::Chunk;
use re_entity_db::{EntityDb, StoreBundle};
use re_log_encoding::decoder::VersionPolicy;
use re_log_types::LogMsg;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Write;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReactionData {
    stable_time: Vec<i64>,
    log_time: Vec<i64>,
    log_tick: Vec<i64>,
    data: Vec<Vec<f64>>,
}

impl Default for ReactionData {
    fn default() -> Self {
        ReactionData {
            stable_time: Vec::new(),
            log_time: Vec::new(),
            log_tick: Vec::new(),
            data: Vec::new(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .init();
    let file_path = "./examples/data/action_r1_h_not_np.rrd";
    let entity_path: &str = "";

    let encoded = File::open(&file_path)?;
    let bundle = StoreBundle::from_rrd(VersionPolicy::Warn, encoded)?;

    let rrd = get_action_entity_db(&bundle);

    let message_iter = rrd.to_messages(None);
    let mut data_map: HashMap<String, ReactionData> = HashMap::new();

    let entity_path_filter = vec![entity_path.to_string()];

    debug!("This rrd file contains {} rows", rrd.num_rows());

    message_iter.for_each(|message| {
        message.iter().for_each(|m| match m {
            LogMsg::ArrowMsg(_store_id, arrow_msg) => match Chunk::from_arrow_msg(&arrow_msg) {
                Ok(chunk) => {
                    debug!(
                        "Chunk ({}) - rows {}, cols {}, comps {}, is_data_chunk {} (tensor: {}, scalar: {})",
                        chunk.entity_path(),
                        chunk.num_rows(),
                        chunk.num_columns(),
                        chunk.num_components(),
                        is_data_chunk(&chunk),
                        is_tensor_chunk(&chunk),
                        is_scalar_chunk(&chunk)
                    );
                    if let Some((key, value)) = chunk.components().first_key_value() {
                        // You now have access to `key` and `value`
                        println!("Key: {:?}, Value: {:?}", key, value);
                        // Do something with key and value
                    } else {
                        println!("No components found in the chunk.");
                    }
                    arrow_msg.schema.fields.iter().for_each(|field| {
                        debug!("- field: {:?}", field);
                    });
                    for (timeline, time_column) in chunk.timelines() {
                        debug!(
                            "Timeline: {:?} {:?} {:?}",
                            timeline.name(),
                            time_column.num_rows(),
                            time_column.times_raw().len()
                        );
                    }
                    if !entity_path_filter.contains(&chunk.entity_path().to_string()) {
                        append_to_data_map(&chunk, &mut data_map);
                    }
                }
                Err(e) => {
                    error!("Error converting ArrowMsg to Chunk: {}", e);
                }
            },
            _ => {
                debug!("This LogMsg is not an ArrowMsg: {:?}", m);
            }
        });
    });

    Ok(())
}

fn is_tensor_chunk(chunk: &Chunk) -> bool {
    // chunk.component_names().for_each(|f| {
    //     debug!("comp names: {:?}", f);
    // });
    chunk
        .component_names()
        .any(|name| name == "rerun.components.TensorData")
}

fn is_scalar_chunk(chunk: &Chunk) -> bool {
    // chunk.component_names().for_each(|f| {
    //     debug!("comp names: {:?}", f);
    // });
    chunk
        .component_names()
        .any(|name| name == "rerun.components.Scalar")
}

fn is_data_chunk(chunk: &Chunk) -> bool {
    is_scalar_chunk(chunk) || is_tensor_chunk(chunk)
}

fn is_action_entity_db(entity_db: &EntityDb) -> bool {
    // Use the iterator provided by entity_paths() and check if any path contains "/viewport" or "/space_view"
    entity_db
        .entity_paths()
        .iter()
        .any(|path| path.to_string().contains("/action"))
}

fn get_action_entity_db(bundle: &StoreBundle) -> &EntityDb {
    bundle
        .entity_dbs()
        .find(|entity_db| is_action_entity_db(entity_db))
        .expect("No EntityDb found with action entity")
}

fn append_to_data_map(chunk: &Chunk, data_map: &mut HashMap<String, ReactionData>) {
    let entity_path = chunk.entity_path().to_string();
    debug!("Entity Path: {}", entity_path);

    let entry = data_map.entry(entity_path).or_default();

    // debug!("chunk: {:?}", chunk);
}
