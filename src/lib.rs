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

/// Retrieve the list of all entity paths from a specific RRD file.
///
/// # Arguments
///
/// * `file_path` - A string slice that holds the path to the RRD file.
///
/// # Returns
///
/// * `PyResult<Vec<String>>` - A list of entity paths if successful, otherwise raises an IOError or ValueError.
///
/// # Example
///
/// ```python
/// entities = requery.list_entity_paths("/path/to/file.rrd")
/// ```
#[pyfunction]
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
pub struct DataChunk {
    entity_path: String,
    timelines: HashMap<String, Py<PyArray1<i64>>>,
    data: Data,
}

#[pyclass]
pub enum Data {
    Tensor { data: Py<PyArray2<Py<PyAny>>> },
    Scalar { data: Py<PyArray1<Py<PyAny>>> },
}

#[pymethods]
impl DataChunk {
    #[new]
    pub fn new(
        py: Python,
        entity_path: String,
        timelines: HashMap<String, Py<PyArray1<i64>>>,
        data: PyObject, // Accept PyObject and determine if it's tensor or scalar data
    ) -> PyResult<Self> {
        // Determine if the provided data is a Tensor or Scalar
        let data_enum = if let Ok(tensor_data) = data.extract::<Py<PyArray2<Py<PyAny>>>>(py) {
            Data::Tensor { data: tensor_data }
        } else if let Ok(scalar_data) = data.extract::<Py<PyArray1<Py<PyAny>>>>(py) {
            Data::Scalar { data: scalar_data }
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Provided data is neither Tensor nor Scalar",
            ));
        };

        Ok(DataChunk {
            entity_path,
            timelines,
            data: data_enum,
        })
    }

    #[getter]
    pub fn entity_path(&self) -> &str {
        &self.entity_path
    }

    #[getter]
    pub fn timelines(&self, py: Python) -> Py<PyDict> {
        let dict_items: Vec<(&str, Py<PyArray1<i64>>)> = self
            .timelines
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();

        dict_items.into_py_dict_bound(py).unbind()
    }

    #[getter]
    pub fn data(&self, py: Python) -> PyObject {
        match &self.data {
            Data::Tensor { data } => data.clone_ref(py).into(),
            Data::Scalar { data } => data.clone_ref(py).into(),
        }
    }
}

impl Default for DataChunk {
    fn default() -> Self {
        Python::with_gil(|py| DataChunk {
            entity_path: String::new(),
            timelines: HashMap::new(),
            data: Data::Tensor {
                data: PyArray2::zeros_bound(py, (0, 0), false).into(),
            },
        })
    }
}

fn to_data_chunk(py: Python, chunk: &Chunk) -> PyResult<Py<DataChunk>> {
    let entity_path = chunk.entity_path().to_string();
    debug!("Entity Path: {}", entity_path);

    // Handle timelines
    let mut timelines = HashMap::new();
    for (timeline, time_column) in chunk.timelines() {
        debug!(
            "Timeline: {:?} {:?} {:?}",
            timeline.name(),
            time_column.num_rows(),
            time_column.times_raw().len()
        );

        let time_array = PyArray1::from_vec_bound(py, time_column.times_raw().to_vec()).unbind();
        timelines.insert(timeline.name().to_string(), time_array);
    }

    // Handle data
    let data = if is_tensor_chunk(chunk) {
        to_tensor_data(py, chunk)?
    } else if is_scalar_chunk(chunk) {
        to_scalar_data(py, chunk)?
    } else {
        return Err(PyErr::new::<PyValueError, _>("Unsupported chunk type"));
    };

    Py::new(
        py,
        DataChunk {
            entity_path,
            timelines,
            data,
        },
    )
}

fn to_tensor_data(py: Python, chunk: &Chunk) -> PyResult<Data> {
    let mut all_rows = Vec::new();

    if let Some((_, tensor_data)) = chunk.components().first_key_value() {
        for i in 0..tensor_data.len() {
            let sub_array = tensor_data.value(i);
            debug!("sub_array: {:?}", sub_array);

            if let Some(struct_array) = sub_array.as_any().downcast_ref::<StructArray>() {
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

        let tensor_array = PyArray2::from_vec2_bound(py, &all_rows)?.unbind();
        Ok(Data::Tensor { data: tensor_array })
    } else {
        Err(PyErr::new::<PyValueError, _>(
            "No tensor data found in chunk",
        ))
    }
}

fn to_scalar_data(py: Python, chunk: &Chunk) -> PyResult<Data> {
    let mut all_rows = Vec::new();

    if let Some((_, scalar_data)) = chunk.components().first_key_value() {
        for i in 0..scalar_data.len() {
            let sub_array = scalar_data.value(i);
            debug!("sub_array: {:?}", sub_array);

            if let Some(scalar_value) = sub_array.as_any().downcast_ref::<Float64Array>() {
                // Assuming the scalar value is a single element in the array
                let value = scalar_value.value(0).into_py(py);
                all_rows.push(value);
            } else {
                error!("Failed to downcast sub_array to Float64Array");
            }
        }

        let scalar_array = PyArray1::from_vec_bound(py, all_rows).unbind();
        Ok(Data::Scalar { data: scalar_array })
    } else {
        Err(PyErr::new::<PyValueError, _>(
            "No scalar data found in chunk",
        ))
    }
}

fn is_tensor_chunk(chunk: &Chunk) -> bool {
    chunk
        .component_names()
        .any(|name| name == "rerun.components.TensorData")
}

fn is_scalar_chunk(chunk: &Chunk) -> bool {
    chunk
        .component_names()
        .any(|name| name == "rerun.components.Scalar")
}

fn is_data_chunk(chunk: &Chunk) -> bool {
    is_scalar_chunk(chunk) || is_tensor_chunk(chunk)
}

// Helper function to match an Array to the correct NumPy array type and convert it to Vec<PyObject>
fn match_array_to_numpy(py: Python, array: &dyn Array) -> PyResult<Vec<PyObject>> {
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

/// Retrieve specific data (scalar or tensor) for an entity in a specific RRD file.
/// Set entity_path to "" will return all the data.
///
/// # Arguments
///
/// * `file_path` - A string slice that holds the path to the RRD file.
/// * `entity_path_filter` - A string slice that holds the specific entity path to filter. Set to empty string to return all data.
/// * `data_type_filter` - A string slice that holds the data type to filter. Set to "scalar" or "tensor" to filter by data type.
///
/// # Returns
///
/// * `PyResult<Py<PyList>>` - A list of ActionChunk objects.
#[pyfunction]
pub fn query_data_entities(
    py: Python<'_>,
    file_path: &str,
    data_type_filter: &str,   // "scalar" or "tensor", or "" for both
    entity_path_filter: &str, // "" for all entities
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

    let mut data_chunks: Vec<Py<DataChunk>> = Vec::new();

    let message_iter = rrd.to_messages(None);
    message_iter.for_each(|message| {
        message.iter().for_each(|m| match m {
            LogMsg::ArrowMsg(_store_id, arrow_msg) => match Chunk::from_arrow_msg(&arrow_msg) {
                Ok(chunk) => {
                    // debug!("Schema: {:#?}", arrow_msg.schema);
                    if matches_data_type(&chunk, data_type_filter)
                        && matches_entity_path(&chunk, entity_path_filter)
                        && is_data_chunk(&chunk)
                    {
                        match to_data_chunk(py, &chunk) {
                            Ok(data_chunk) => {
                                data_chunks.push(data_chunk);
                            }
                            Err(e) => println!("Failed calling to_data_chunk: {:?}", e),
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

    if data_chunks.is_empty() {
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "No data found for the specified entity and data type",
        ))
    } else {
        let py_data_chunks = PyList::new_bound(py, &data_chunks).unbind();
        Ok(py_data_chunks)
    }
}

fn matches_data_type(chunk: &Chunk, data_type_filter: &str) -> bool {
    match data_type_filter {
        "scalar" => is_scalar_chunk(chunk),
        "tensor" => is_tensor_chunk(chunk),
        "" => true, // No filter, accept all data types
        _ => false, // Invalid filter, nothing fits
    }
}

fn matches_entity_path(chunk: &Chunk, entity_path_filter: &str) -> bool {
    if entity_path_filter.is_empty() {
        true // No filter, accept all entity paths
    } else {
        chunk.entity_path().to_string().contains(entity_path_filter)
    }
}

#[pyclass]
/// A class representing a text chunk extracted from the RRD file.
///
/// # Fields
///
/// * `entity_path` - The path of the entity associated with this chunk.
/// * `media_type` - The media type of the metadata, such as text/plain.
/// * `text` - The metadata.
///
/// This class is subject to change in the future, as data types are being extended.
pub enum MetaChunk {
    Text {
        entity_path: String,
        media_type: String,
        text: String,
    },
}

// Implement the conversion to Py<MetaChunk>
impl IntoPy<Py<MetaChunk>> for MetaChunk {
    fn into_py(self, py: Python) -> Py<MetaChunk> {
        Py::new(py, self).unwrap() // Adjust error handling as needed
    }
}
#[pymethods]
impl MetaChunk {
    #[new]
    pub fn new(entity_path: String, media_type: String, text: String) -> Self {
        MetaChunk::Text {
            entity_path,
            media_type,
            text,
        }
    }

    #[getter]
    pub fn entity_path(&self) -> &str {
        let MetaChunk::Text { entity_path, .. } = self;
        entity_path
    }

    #[getter]
    pub fn media_type(&self) -> &str {
        let MetaChunk::Text { media_type, .. } = self;
        media_type
    }

    #[getter]
    pub fn text(&self) -> &str {
        let MetaChunk::Text { text, .. } = self;
        text
    }

    fn __repr__(&self) -> PyResult<String> {
        let MetaChunk::Text {
            entity_path,
            media_type,
            text,
        } = self;

        Ok(format!(
            "<MetaChunk(entity_path='{}', media_type='{}', text='{}')>",
            entity_path, media_type, text
        ))
    }
}

fn is_meta_chunk(chunk: &Chunk) -> bool {
    chunk
        .component_names()
        .any(|name| name == "rerun.components.Text")
}

/// Retrieve specific metadata for an entity in a specific RRD file.
/// Set entity_path to "" will return all the data.
///
/// # Arguments
///
/// * `file_path` - A string slice that holds the path to the RRD file.
/// * `entity_path` - A string slice that holds the specific entity path to filter. Set to empty string to return all data.
///
/// # Returns
///
/// * `PyResult<Py<PyList>>` - A list of MetaChunk objects.
#[pyfunction]
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

    let meta_chunk = MetaChunk::Text {
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

    m.add_function(wrap_pyfunction!(query_data_entities, m)?)?;
    m.add_function(wrap_pyfunction!(query_meta_entities, m)?)?;
    m.add_function(wrap_pyfunction!(list_entity_paths, m)?)?;
    Ok(())
}
