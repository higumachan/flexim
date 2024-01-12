import logging
from itertools import batched
from typing import Any

import grpc
from grpc import Channel
from pydantic import BaseModel, ConfigDict

from flexim.data_type import ImageData, DataFrameData, Tensor2DData
from flexim.pb import connect_pb2, connect_pb2_grpc


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
                data_type=connect_pb2.DataType.Image
            ),
        )] + [connect_pb2.AppendDataRequest(
            data_bytes=bytes(list(chunked_data))
        ) for chunked_data in batched(data_bytes, CHUNK_SIZE)])

    response: connect_pb2.AppendDataResponse = stub.AppendData(
        data_iter
    )

    print(response)
