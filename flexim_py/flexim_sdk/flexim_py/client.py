import logging
from typing import Any

import grpc
from grpc import Channel
from pydantic import BaseModel, ConfigDict

from flexim_py.data_type import ImageData, DataFrameData, Tensor2DData, SpecialColumn
from flexim_py.pb import connect_pb2, connect_pb2_grpc
from flexim_py.utility import batched

logger = logging.getLogger(__name__)

CHUNK_SIZE = 1024 * 1024


class Client(BaseModel):
    host: str
    port: int
    channel: Channel | None

    model_config = ConfigDict(
        arbitrary_types_allowed=True
    )


global_client: Client | None = None


def init(host: str, port: int):
    global global_client
    channel = grpc.insecure_channel(f"{host}:{port}")
    global_client = Client(host=host, port=port, channel=channel)


def create_bag(name: str) -> int:

    global global_client

    stub = connect_pb2_grpc.FleximConnectStub(global_client.channel)
    response: connect_pb2.CreateBagResponse = stub.CreateBag(connect_pb2.CreateBagRequest(name=name))

    return response.id


def append_data(bag_id: int, name: str, data: ImageData | DataFrameData | Tensor2DData):
    global global_client

    data_bytes = data.to_bytes()

    stub = connect_pb2_grpc.FleximConnectStub(global_client.channel)

    data_iter = iter([connect_pb2.AppendDataRequest(
            meta=connect_pb2.AppendDataRequest.DataMeta(
                bag_id=bag_id,
                name=name,
                data_type=_data_type_to_proto(data)
            ),
        )] + [connect_pb2.AppendDataRequest(
            data_bytes=bytes(list(chunked_data))
        ) for chunked_data in batched(data_bytes, CHUNK_SIZE)])

    response: connect_pb2.AppendDataResponse = stub.AppendData(
        data_iter
    )

    print(response)


def _data_type_to_proto(data: ImageData | DataFrameData | Tensor2DData) -> connect_pb2.DataType:
    if isinstance(data, ImageData):
        return connect_pb2.DataType.Image
    elif isinstance(data, DataFrameData):
        return connect_pb2.DataType.DataFrame
    elif isinstance(data, Tensor2DData):
        return connect_pb2.DataType.Tensor2D
    else:
        raise RuntimeError(f"Unknown data type {type(data)}")

def _dataframe_special_columns(data: DataFrameData) -> dict[str, connect_pb2.SpecialColumn]:
    return {
        key: _special_column_to_proto(value)
        for key, value in data.special_columns.items()
    }


def _special_column_to_proto(special_column: SpecialColumn) -> connect_pb2.SpecialColumn:
    match special_column:
        case SpecialColumn.Rectangle:
            return connect_pb2.SpecialColumn.Rectangle
        case SpecialColumn.Segment:
            return connect_pb2.SpecialColumn.Segment
        case _:
            raise RuntimeError(f"Unknown special column {special_column}")
