from pydantic import BaseModel

from flexim_py.client import create_bag, append_data
from flexim_py.data_type import ImageData, DataFrameData, Tensor2DData


class Bag(BaseModel):
    connected_id: int | None = None
    name: str

    def __str__(self):
        return f"Bag(id={self.connected_id}, name={self.name})"

    def __enter__(self):
        id = create_bag(self.name)
        self.connected_id = id
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.connected_id = None

    def append_data(self, name: str, data: ImageData | DataFrameData | Tensor2DData):
        if self.connected_id is None:
            raise RuntimeError("Bag is not connected")
        append_data(self.connected_id, name, data)
