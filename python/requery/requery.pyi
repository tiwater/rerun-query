from typing import Any, Dict, List, Union

class DataChunk:
    """
    Represents a chunk of data associated with an entity path and timelines.

    Attributes:
        entity_path (str): The path of the entity.
        timelines (Dict[str, Any]): A dictionary containing timelines.
        data (Union[Data.Tensor, Data.Scalar]): The data associated with the entity.
    """

    def __init__(
        self,
        entity_path: str,
        timelines: Dict[str, Any],
        data: Union["Data.Tensor", "Data.Scalar"],
    ) -> None: ...
    @property
    def entity_path(self) -> str:
        """Get the entity path."""
        ...

    @property
    def timelines(self) -> Dict[str, Any]:
        """Get the timelines dictionary."""
        ...

    @property
    def data(self) -> Union["Data.Tensor", "Data.Scalar"]:
        """Get the data associated with the entity."""
        ...

class Data:
    class Tensor:
        """Represents tensor data."""

        data: Any

        def __init__(self, data: Any) -> None: ...

    class Scalar:
        """Represents scalar data."""

        data: Any

        def __init__(self, data: Any) -> None: ...

class MetaChunk:
    """
    Represents a chunk of metadata.

    Attributes:
        entity_path (str): The path of the entity.
        media_type (str): The media type of the chunk.
        text (str): The text content of the chunk.
    """

    def __init__(self, entity_path: str, media_type: str, text: str) -> None: ...
    @property
    def entity_path(self) -> str:
        """Get the entity path."""
        ...

    @property
    def media_type(self) -> str:
        """Get the media type."""
        ...

    @property
    def text(self) -> str:
        """Get the text content."""
        ...

    def __repr__(self) -> str:
        """Return a string representation of the MetaChunk."""
        ...

def list_entity_paths(file_path: str) -> List[str]:
    """
    Retrieve the list of all entity paths from a specific RRD file.

    Args:
        file_path (str): The path to the RRD file.

    Returns:
        List[str]: A list of entity paths.

    Raises:
        IOError: If there's an issue reading the file.
        ValueError: If the file format is invalid.

    Example:
        >>> paths = list_entity_paths("/path/to/data.rrd")
        >>> print(paths)
        ['entity1', 'entity2', 'entity3']
    """
    ...

def query_data_entities(
    file_path: str,
    data_type_filter: str = "",
    entity_path_filter: str = "",
) -> List[DataChunk]:
    """
    Retrieve specific data (scalar or tensor) for entities in a specific RRD file.

    Args:
        file_path (str): The path to the RRD file.
        data_type_filter (str, optional): The data type to filter. Use "scalar" or "tensor", or "" for both. Defaults to "".
        entity_path_filter (str, optional): The specific entity path to filter. Use "" for all entities. Defaults to "".

    Returns:
        List[DataChunk]: A list of DataChunk objects.

    Raises:
        IOError: If there's an issue reading the file.
        ValueError: If the file format is invalid or if an invalid data_type_filter is provided.

    Example:
        >>> chunks = query_data_entities("/path/to/data.rrd", data_type_filter="tensor", entity_path_filter="entity1")
        >>> for chunk in chunks:
        ...     print(f"Entity: {chunk.entity_path}, Data: {chunk.data}")
    """
    ...

def query_meta_entities(file_path: str, entity_path: str = "") -> List[MetaChunk]:
    """
    Retrieve specific metadata for entities in a specific RRD file.

    Args:
        file_path (str): The path to the RRD file.
        entity_path (str, optional): The specific entity path to filter. Use "" for all entities. Defaults to "".

    Returns:
        List[MetaChunk]: A list of MetaChunk objects.

    Raises:
        IOError: If there's an issue reading the file.
        ValueError: If the file format is invalid.

    Example:
        >>> meta_chunks = query_meta_entities("/path/to/data.rrd", entity_path="entity1")
        >>> for chunk in meta_chunks:
        ...     print(f"Entity: {chunk.entity_path}, Media Type: {chunk.media_type}, Text: {chunk.text}")
    """
    ...
