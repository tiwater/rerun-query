use env_logger;
use log::{debug, error};
use numpy::{PyArray1, PyArray2};
use pyo3::{
    exceptions::{PyIOError, PyValueError},
    prelude::*,
    types::{IntoPyDict, PyDict, PyList},
};
use re_arrow2::array::{Array, Float64Array, ListArray, StructArray, UnionArray, Utf8Array};
use re_chunk::{Chunk, ComponentName};
use re_entity_db::{EntityDb, StoreBundle};
use re_log_encoding::decoder::VersionPolicy;
use re_log_types::LogMsg;
use std::{collections::HashMap, fs::File};
// pub async fn get_entity_dbs(
//     data: &web::Data<AppState>,
//     rrd_name: &str,
// ) -> Result<Vec<String>, String> {
//     let file_path = data.folder_path.join(rrd_name);
//     let encoded = File::open(&file_path).map_err(|e| e.to_string())?;
//     let bundle = StoreBundle::from_rrd(re_log_encoding::decoder::VersionPolicy::Warn, encoded)
//         .map_err(|e| e.to_string())?;

//     let mut entity_dbs = Vec::new();
//     for rrd in bundle.entity_dbs() {
//         entity_dbs.push(rrd.to_string());
//     }

//     Ok(entity_dbs)
// }

#[pyfunction]
/// Retrieve a list of entities from a specific RRD file.
pub fn list_entity_paths(file_path: &str) -> PyResult<Vec<String>> {
    let encoded = File::open(&file_path)
        .map_err(|e| PyErr::new::<PyIOError, _>(format!("File open error: {}", e)))?;
    let bundle: StoreBundle = StoreBundle::from_rrd(VersionPolicy::Warn, encoded)
        .map_err(|e| PyErr::new::<PyValueError, _>(format!("Decoding error: {}", e)))?;

    let mut entities = Vec::new();
    for rrd in bundle.entity_dbs() {
        for entity in rrd.entity_paths() {
            entities.push(entity.to_string());
        }
    }

    Ok(entities)
}

// fn is_blueprint_entity_db(entity_db: &EntityDb) -> bool {
//     // Use the iterator provided by entity_paths() and check if any path contains "/viewport" or "/space_view"
//     entity_db.entity_paths().iter().any(|path| {
//         path.to_string().contains("/viewport") || path.to_string().contains("/space_view")
//     })
// }

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

#[pyclass]
pub struct ActionChunk {
    entity_path: String,
    timelines: HashMap<String, Py<PyArray1<i64>>>,
    data: Py<PyArray2<Py<PyAny>>>,
}

#[pymethods]
impl ActionChunk {
    #[new]
    pub fn new(
        _py: Python,
        entity_path: String,
        timelines: HashMap<String, Py<PyArray1<i64>>>,
        data: Py<PyArray2<Py<PyAny>>>,
    ) -> Self {
        ActionChunk {
            entity_path,
            timelines,
            data,
        }
    }

    #[getter]
    pub fn entity_path(&self) -> &str {
        &self.entity_path
    }

    #[getter]
    pub fn timelines(&self, py: Python) -> Py<PyDict> {
        // Convert the Rust HashMap to a Vec of tuples, which can be converted into a PyDict
        let dict_items: Vec<(&str, Py<PyArray1<i64>>)> = self
            .timelines
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();

        // Convert the Vec of tuples into a PyDict using into_py_dict_bound
        let dict = dict_items.into_py_dict_bound(py).unbind();

        dict
    }

    #[getter]
    pub fn data(&self, py: Python) -> Py<PyArray2<Py<PyAny>>> {
        self.data.clone_ref(py)
    }
}

impl Default for ActionChunk {
    fn default() -> Self {
        Python::with_gil(|py| ActionChunk {
            entity_path: String::new(),
            timelines: HashMap::new(),
            data: PyArray2::zeros_bound(py, (0, 0), false).into(),
        })
    }
}
// Function to convert a Chunk to ActionChunk
fn to_action_chunk(py: Python, chunk: &Chunk) -> PyResult<Py<ActionChunk>> {
    let entity_path = chunk.entity_path().to_string();
    debug!("Entity Path: {}", entity_path);

    // Creating a new default ActionChunk
    let mut action_chunk = ActionChunk::default();
    action_chunk.entity_path = entity_path;
    let mut all_rows = Vec::new(); // Collect all 1D arrays

    // Iterating over timelines
    for (timeline, time_column) in chunk.timelines() {
        debug!(
            "Timeline: {:?} {:?} {:?}",
            timeline.name(),
            time_column.num_rows(),
            time_column.times_raw().len()
        );

        // Convert time_column.times_raw() to a PyArray1
        let time_array = PyArray1::from_vec_bound(py, time_column.times_raw().to_vec()).unbind();

        // Replace the corresponding timeline entry in the action_chunk
        action_chunk
            .timelines
            .insert(timeline.name().to_string(), time_array);
    }

    // Get the tensor component from the chunk and collect into a 2D array
    if let Some((_, tensor_data)) = chunk.components().first_key_value() {
        for i in 0..tensor_data.len() {
            let sub_array = tensor_data.value(i);
            debug!("sub_array: {:?}", sub_array);

            if let Some(struct_array) = sub_array.as_any().downcast_ref::<StructArray>() {
                // Accessing the 'buffer' field as the second field in StructArray
                if let Some(buffer_array) = struct_array.values().get(1) {
                    let row = match_array_to_numpy(py, buffer_array.as_ref())?;
                    all_rows.push(row);
                } else {
                    error!("Buffer field not found in StructArray");
                }
            } else {
                error!("Failed to downcast sub_array to StructArray");
            }
        }

        // Convert Vec<PyObject> to a 2D NumPy array
        let tensor_array = PyArray2::from_vec2_bound(py, &all_rows)?.unbind();
        action_chunk.data = tensor_array.to_owned();
    }

    Py::new(py, action_chunk)
}

// Helper function to match an Array to the correct NumPy array type and convert it to Vec<PyObject>
fn match_array_to_numpy(py: Python, array: &dyn Array) -> PyResult<Vec<PyObject>> {
    debug!("array to match: {:?}", array);
    if let Some(union_array) = array.as_any().downcast_ref::<UnionArray>() {
        let mut result: Vec<Py<PyAny>> = Vec::with_capacity(union_array.len());
        let fields = union_array.fields();
        let type_ids = union_array.types();
        let offsets = union_array.offsets();

        for i in 0..union_array.len() {
            let field_type = type_ids[i];
            let value_index = match offsets {
                Some(offset_buffer) => offset_buffer[i] as usize, // Using the offset if available
                None => i, // If no offsets, use the index directly
            };
            let value = match field_type {
                11 => {
                    let list_array = fields[11]
                        .as_any()
                        .downcast_ref::<ListArray<i32>>()
                        .unwrap();
                    let list_element: Box<dyn Array> = list_array.value(value_index);
                    debug!("List Element {}: {:?}", value_index, list_element);
                    let struct_array = list_element
                        .as_any()
                        .downcast_ref::<Float64Array>()
                        .unwrap();

                    let py_array = struct_array.values().to_vec().into_py(py);
                    py_array
                }
                _ => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Unsupported type in UnionArray {}",
                        field_type,
                    )));
                }
            };

            result.push(value);
        }

        Ok(result)
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected UnionArray but got a different array type",
        ))
    }
}

fn is_action_chunk(chunk: &Chunk) -> bool {
    // chunk.component_names().for_each(|f| {
    //     debug!("comp names: {:?}", f);
    // });
    chunk
        .component_names()
        .any(|name| name == "rerun.components.TensorData")
}

/// Retrieve specific data (scalar or tensor) for an entity in a specific RRD file.
/// Set entity_path to "" will return all the data.
#[pyfunction]
pub fn query_action_entities(
    py: Python<'_>,
    file_path: &str,
    entity_path: &str,
) -> PyResult<Py<PyList>> {
    let encoded = File::open(&file_path).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("File open error: {}", e))
    })?;
    let bundle: StoreBundle =
        StoreBundle::from_rrd(re_log_encoding::decoder::VersionPolicy::Warn, encoded).map_err(
            |e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Decoding error: {}", e)),
        )?;

    let rrd = get_action_entity_db(&bundle);
    debug!("This rrd file contains {} rows", rrd.num_rows());

    let mut action_chunks: Vec<Py<ActionChunk>> = Vec::new();

    let message_iter = rrd.to_messages(None);
    message_iter.for_each(|message| {
        message.iter().for_each(|m| match m {
            LogMsg::ArrowMsg(_store_id, arrow_msg) => match Chunk::from_arrow_msg(&arrow_msg) {
                Ok(chunk) => {
                    // debug!("Schema: {:#?}", arrow_msg.schema);
                    if (entity_path.is_empty() || (entity_path == &chunk.entity_path().to_string()))
                        && is_action_chunk(&chunk)
                    {
                        match to_action_chunk(py, &chunk) {
                            Ok(action_chunk) => {
                                action_chunks.push(action_chunk);
                            }
                            Err(e) => println!("Failed calling to_action_chunk: {:?}", e),
                        }
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

    if action_chunks.is_empty() {
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "No data found for the specified entity and data type",
        ))
    } else {
        let py_action_chunks = PyList::new_bound(py, &action_chunks).unbind();
        Ok(py_action_chunks)
    }
}

#[pyclass]
pub struct MetaChunk {
    entity_path: String,
    media_type: String,
    text: String,
}

#[pymethods]
impl MetaChunk {
    #[new]
    pub fn new() -> Self {
        Self {
            entity_path: String::new(),
            media_type: String::new(),
            text: String::new(),
        }
    }

    // Getter for entity_path
    #[getter]
    pub fn entity_path(&self) -> &str {
        &self.entity_path
    }

    // Getter for media_type
    #[getter]
    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    // Getter for text
    #[getter]
    pub fn text(&self) -> &str {
        &self.text
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "<MetaChunk(entity_path='{}', media_type={}, text={})>",
            self.entity_path, self.media_type, self.text
        ))
    }
}

fn is_meta_chunk(chunk: &Chunk) -> bool {
    chunk
        .component_names()
        .any(|name| name == "rerun.components.Text")
}

/// Retrieve specific media type data from a specific RRD file.
#[pyfunction]
/// Retrieve specific media type data from a specific RRD file.
pub fn query_meta_entities(
    py: Python<'_>,
    file_path: &str,
    entity_path: &str,
) -> PyResult<Py<PyList>> {
    let encoded = File::open(&file_path)
        .map_err(|e| PyErr::new::<PyIOError, _>(format!("File open error: {}", e)))?;
    let bundle: StoreBundle = StoreBundle::from_rrd(VersionPolicy::Warn, encoded)
        .map_err(|e| PyErr::new::<PyValueError, _>(format!("Decoding error: {}", e)))?;

    let rrd = get_action_entity_db(&bundle);

    let mut meta_chunks: Vec<Py<MetaChunk>> = Vec::new();

    let message_iter = rrd.to_messages(None);
    message_iter.for_each(|message| {
        message.iter().for_each(|m| match m {
            LogMsg::ArrowMsg(_store_id, arrow_msg) => match Chunk::from_arrow_msg(&arrow_msg) {
                Ok(chunk) => {
                    if (entity_path.is_empty() || entity_path == &chunk.entity_path().to_string())
                        && is_meta_chunk(&chunk)
                    {
                        debug!("Meta chunk: {:?}", chunk);
                        match to_meta_chunk(py, &chunk) {
                            Ok(meta_chunk) => {
                                meta_chunks.push(meta_chunk);
                            }
                            Err(e) => println!("Failed calling to_meta_chunk: {:?}", e),
                        }
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

    if meta_chunks.is_empty() {
        Err(PyErr::new::<PyValueError, _>(
            "No meta entities found for the specified entity and data type",
        ))
    } else {
        let py_meta_chunks = PyList::new_bound(py, &meta_chunks).unbind();
        Ok(py_meta_chunks)
    }
}

fn to_meta_chunk(py: Python, chunk: &Chunk) -> PyResult<Py<MetaChunk>> {
    let entity_path = chunk.entity_path().to_string();

    let media_type_component = ComponentName::from("rerun.components.MediaType");
    let text_component = ComponentName::from("rerun.components.Text");

    // Extract the media type
    let media_type_array = chunk
        .components()
        .get(&media_type_component)
        .and_then(|array| array.as_any().downcast_ref::<ListArray<i32>>());

    let media_type = if let Some(media_type_array) = media_type_array {
        media_type_array
            .value(0)
            .as_any()
            .downcast_ref::<Utf8Array<i32>>()
            .unwrap()
            .value(0)
            .to_string()
    } else {
        "unknown".to_string()
    };

    // Extract the text data
    let text_array = chunk
        .components()
        .get(&text_component)
        .and_then(|array| array.as_any().downcast_ref::<ListArray<i32>>());

    let text = if let Some(text_array) = text_array {
        text_array
            .value(0)
            .as_any()
            .downcast_ref::<Utf8Array<i32>>()
            .unwrap()
            .value(0)
            .to_string()
    } else {
        "no text".to_string()
    };

    let meta_chunk = MetaChunk {
        entity_path,
        media_type,
        text,
    };

    Py::new(py, meta_chunk)
}

/// A Python module implemented in Rust.
/// This module is a plugin for the Python package `rerun-query`.
#[pymodule]
fn requery(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    env_logger::init();

    m.add_function(wrap_pyfunction!(query_action_entities, m)?)?;
    m.add_function(wrap_pyfunction!(query_meta_entities, m)?)?;
    m.add_function(wrap_pyfunction!(list_entity_paths, m)?)?;
    Ok(())
}
