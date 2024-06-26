import json
from enum import Enum
from io import BytesIO
from typing import Self, Literal, Any
from flexim_py import _flexim_py_lib

import PIL.Image
import pyarrow
from pydantic import BaseModel, ConfigDict
import numpy.typing as npt
import numpy as np
import pandas


class SpecialColumn(str, Enum):
    Rectangle = "Rectangle"
    Segment = "Segment"
    Color = "Color"


class Rectangle(BaseModel):
    x1: float
    y1: float
    x2: float
    y2: float


class Segment(BaseModel):
    x1: float
    y1: float
    x2: float
    y2: float


class Color(BaseModel):
    r: float
    g: float
    b: float


class ImageData(BaseModel):
    type: Literal["Image"] = "Image"
    image: npt.NDArray[np.uint8]

    model_config = ConfigDict(arbitrary_types_allowed=True)

    @classmethod
    def from_numpy(cls, array: npt.NDArray[np.uint8]) -> Self:
        return ImageData(image=array)

    @classmethod
    def from_pil(cls, image: PIL.Image.Image) -> Self:
        return ImageData(image=np.array(image))

    def to_bytes(self) -> bytes:
        # bytes encoded as png
        img_bytes = BytesIO()
        PIL.Image.fromarray(self.image).save(img_bytes, format="PNG")
        return img_bytes.getvalue()


class DataFrameData(BaseModel):
    type: Literal["DataFrame"] = "DataFrame"
    dataframe: pandas.DataFrame
    special_columns: dict[str, SpecialColumn]

    model_config = ConfigDict(arbitrary_types_allowed=True)

    @classmethod
    def from_pandas(cls, dataframe: pandas.DataFrame, special_columns: dict[str, SpecialColumn]):
        return cls(dataframe=dataframe, special_columns=special_columns)

    def to_bytes(self) -> bytes:
        sink = BytesIO()
        pa_df = pyarrow.Table.from_pandas(self.dataframe)
        with pyarrow.ipc.new_file(sink, pa_df.schema) as writer:
            writer.write(pa_df)
        return sink.getvalue()


class Tensor2DData(BaseModel):
    type: Literal["Tensor2D"] = "Tensor2D"
    tensor: npt.NDArray[np.float32]
    offset: tuple[int, int]

    model_config = ConfigDict(arbitrary_types_allowed=True)

    @classmethod
    def from_numpy(cls, array: npt.NDArray[np.float32]):
        return cls(tensor=array, offset=(0, 0))

    @classmethod
    def from_numpy_with_offset(cls, array: npt.NDArray[np.float32], offset: tuple[int, int]):
        return cls(tensor=array, offset=offset)

    def to_bytes(self) -> bytes:
        # bytes encoded as C major
        return _flexim_py_lib.tensor2d_to_bytes(self.tensor, self.offset)


class ObjectData(BaseModel):
    type: Literal["Object"] = "Object"
    object: object

    model_config = ConfigDict(arbitrary_types_allowed=True)

    @classmethod
    def from_object(cls, obj: Any):
        return cls(object=obj)

    @classmethod
    def from_pydantic(cls, obj: BaseModel):
        return cls(object=obj.model_dump())

    def to_bytes(self) -> bytes:
        # bytes encoded as json
        return json.dumps(self.object).encode("utf-8")
