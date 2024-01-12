from enum import Enum
from io import BytesIO
from typing import Self

import PIL.Image
from pydantic import BaseModel, ConfigDict
import numpy.typing as npt
import numpy as np
import pandas


class SpecialColumn(str, Enum):
    Rectangle = "Rectangle"


class ImageData(BaseModel):
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
    dataframe: pandas.DataFrame
    special_columns: dict[str, SpecialColumn]

    model_config = ConfigDict(arbitrary_types_allowed=True)

    def from_pandas(self, dataframe: pandas.DataFrame, special_columns: dict[str, SpecialColumn]):
        self.dataframe = dataframe
        self.special_columns = special_columns

    def to_bytes(self) -> bytes:
        # bytes encoded as csv
        return self.dataframe.to_csv().encode("utf-8")


class Tensor2DData(BaseModel):
    tensor: npt.NDArray[np.float32]

    model_config = ConfigDict(arbitrary_types_allowed=True)

    def from_numpy(self, array: npt.NDArray[np.float32]):
        self.tensor = array
        assert self.tensor.ndim == 2

    def to_bytes(self) -> bytes:
        # bytes encoded as C major
        return self.tensor.tobytes("C")
