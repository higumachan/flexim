import logging
import subprocess
from pathlib import Path
from typing import Any, Type, TypeVar

import grpc
import pydantic
from grpc import Channel
from pydantic import BaseModel, ConfigDict

from flexim_py.data_type import ImageData, DataFrameData, Tensor2DData, SpecialColumn, Rectangle, Segment, Color, \
    ObjectData
from flexim_py.pb import connect_pb2, connect_pb2_grpc
from flexim_py.utility import batched
from flexim_py._flexim_py_lib import start_localstorage_server

logger = logging.getLogger(__name__)

CHUNK_SIZE = 1024 * 1024


class Client(BaseModel):
    host: str
    port: int
    channel: Channel | None

    model_config = ConfigDict(arbitrary_types_allowed=True)


class Server(BaseModel):
    handle: int


global_client: Client | None = None
global_server: Server | None = None


def init(host: str, port: int):
    init_server(host, port)


def init_server(host: str, port: int):
    global global_client
    channel = grpc.insecure_channel(f"{host}:{port}")
    global_client = Client(host=host, port=port, channel=channel)


def init_localstorage(base_directory: Path):
    global global_client, global_server

    port = 50111
    start_localstorage_server(str(base_directory), port)
    channel = grpc.insecure_channel(f"localhost:{port}")
    global_client = Client(host="localhost", port=port, channel=channel)


def create_bag(name: str) -> int:
    global global_client

    stub = connect_pb2_grpc.FleximConnectStub(global_client.channel)
    response: connect_pb2.CreateBagResponse = stub.CreateBag(connect_pb2.CreateBagRequest(name=name))

    return response.id


def append_data(bag_id: int, name: str, data: ImageData | DataFrameData | Tensor2DData):
    global global_client

    if not _validate_data(data):
        raise ValueError(f"Data is not valid")

    data_bytes = data.to_bytes()

    stub = connect_pb2_grpc.FleximConnectStub(global_client.channel)

    data_iter = iter(
        [
            connect_pb2.AppendDataRequest(
                meta=connect_pb2.AppendDataRequest.DataMeta(
                    bag_id=bag_id,
                    name=name,
                    data_type=_data_type_to_proto(data),
                    special_columns=_dataframe_special_columns(data) if data.type == "DataFrame" else {},
                ),
            )
        ]
        + [connect_pb2.AppendDataRequest(data_bytes=bytes(list(chunked_data))) for chunked_data in
           batched(data_bytes, CHUNK_SIZE)]
    )

    _response: connect_pb2.AppendDataResponse = stub.AppendData(data_iter)


def _data_type_to_proto(
        data: ImageData | DataFrameData | Tensor2DData | ObjectData,
) -> connect_pb2.DataType:
    if isinstance(data, ImageData):
        return connect_pb2.DataType.Image
    elif isinstance(data, DataFrameData):
        return connect_pb2.DataType.DataFrame
    elif isinstance(data, Tensor2DData):
        return connect_pb2.DataType.Tensor2D
    elif isinstance(data, ObjectData):
        return connect_pb2.DataType.Object
    else:
        raise RuntimeError(f"Unknown data type {type(data)}")


def _dataframe_special_columns(
        data: DataFrameData,
) -> dict[str, connect_pb2.AppendDataRequest.DataMeta.SpecialColumn]:
    return {key: _special_column_to_proto(value) for key, value in data.special_columns.items()}


def _special_column_to_proto(
        special_column: SpecialColumn,
) -> connect_pb2.AppendDataRequest.DataMeta.SpecialColumn:
    match special_column:
        case SpecialColumn.Rectangle:
            return connect_pb2.AppendDataRequest.DataMeta.SpecialColumn.Rectangle
        case SpecialColumn.Segment:
            return connect_pb2.AppendDataRequest.DataMeta.SpecialColumn.Segment
        case SpecialColumn.Color:
            return connect_pb2.AppendDataRequest.DataMeta.SpecialColumn.Color
        case _:
            raise RuntimeError(f"Unknown special column {special_column}")


def _validate_value_with_type(value: Any, value_type: type[BaseModel]) -> bool:
    model_validated = True
    try:
        value_type.model_validate(value)
    except pydantic.ValidationError:
        model_validated = False
    model_json_validated = True
    try:
        value_type.model_validate(value)
    except pydantic.ValidationError:
        model_json_validated = False
    return value is None or isinstance(value, value_type) or model_validated or model_json_validated


def _validate_value(value: Any, special_column: SpecialColumn) -> bool:
    match special_column:
        case SpecialColumn.Rectangle:
            return _validate_value_with_type(value, Rectangle)
        case SpecialColumn.Segment:
            return _validate_value_with_type(value, Segment)
        case SpecialColumn.Color:
            return _validate_value_with_type(value, Color)


def _validate_data(data: ImageData | DataFrameData | Tensor2DData):
    if data.type == "Image":
        return data.image.ndim == 3 and (data.image.shape[-1] == 3 or data.image.shape[-1] == 4)
    elif data.type == "DataFrame":
        special_columns = data.special_columns
        for key, sp_value in special_columns.items():
            return data.dataframe[key].map(lambda value: _validate_value(value, sp_value)).all()
    elif data.type == "Tensor2D":
        return data.tensor.ndim == 2
    elif data.type == "Object":
        return True
    else:
        return False
