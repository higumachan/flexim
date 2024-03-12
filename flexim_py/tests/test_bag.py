import subprocess
from io import BytesIO
from pathlib import Path
from tempfile import mkdtemp

import numpy as np
import pandas
import pyarrow
import pyarrow.ipc
import pytest
from PIL import Image

from flexim_py.bag import Bag
from flexim_py.client import init, init_localstorage
from flexim_py.data_type import (
    ImageData,
    Rectangle,
    DataFrameData,
    Segment,
    SpecialColumn, Tensor2DData, Color,
)

test_df = pandas.DataFrame(
    [
        {
            "a": 1,
            "b": 2,
            "c": Rectangle(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump(),
            "d": Segment(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump(),
        },
        {
            "a": 3,
            "b": 4,
            "c": Rectangle(x1=100.0, y1=50.0, x2=200.0, y2=300.0).model_dump(),
            "d": Segment(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump(),
        },
        {
            "a": 5,
            "b": 6,
            "c": Rectangle(x1=400.0, y1=50.0, x2=100.0, y2=500.0).model_dump(),
            "d": Segment(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump(),
        },
    ]
)

test_df_with_null = pandas.DataFrame(
    [
        {"a": 1, "b": 2, "c": None, "d": None},
        {
            "a": 3,
            "b": 4,
            "c": Rectangle(x1=100.0, y1=50.0, x2=200.0, y2=300.0).model_dump(),
            "d": Segment(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump(),
        },
        {
            "a": 5,
            "b": 6,
            "c": Rectangle(x1=400.0, y1=50.0, x2=500.0, y2=500.0).model_dump(),
            "d": Segment(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump(),
        },
    ]
)


test_df_with_invalid = pandas.DataFrame(
    [
        {"a": 1, "b": 2, "c": None, "d": None},
        {
            "a": 3,
            "b": 4,
            "c": {"x": 100, "y": 100},
            "d": Segment(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump(),
        },
        {
            "a": 5,
            "b": 6,
            "c": Rectangle(x1=400.0, y1=50.0, x2=500.0, y2=500.0).model_dump(),
            "d": {"x": 100, "y": 100},
        },
    ]
)


test_df_with_color = pandas.DataFrame(
    [
        {"a": 1, "color": Color(r=255, g=0, b=0).model_dump(), "rect": Rectangle(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump()},
        {"a": 2, "color": Color(r=0, g=255, b=0).model_dump(), "rect": Rectangle(x1=200.0, y1=100.0, x2=300.0, y2=200.0).model_dump()},
        {"a": 3, "color": Color(r=0, g=0, b=255).model_dump() , "rect": Rectangle(x1=100.0, y1=400.0, x2=200.0, y2=500.0).model_dump()},
    ]
)


@pytest.fixture(autouse=True)
def init_client():
    init_localstorage(Path(mkdtemp()))


def test_simple_append_data():
    # Create a bag
    with Bag(name="test_bag") as bag:
        # Append data
        bag.append_data(
            "python-image-data",
            ImageData.from_pil(Image.open("../assets/flexim-logo-1.png")),
        )
        bag.append_data(
            "python-table-data",
            DataFrameData.from_pandas(
                test_df,
                {
                    "c": SpecialColumn.Rectangle,
                    "d": SpecialColumn.Segment,
                },
            ),
        )


def test_append_data_with_null():
    with Bag(name="test_bag_with_null") as bag:
        # Append data
        bag.append_data(
            "python-table-data",
            DataFrameData.from_pandas(
                test_df_with_null,
                {
                    "c": SpecialColumn.Rectangle,
                    "d": SpecialColumn.Segment,
                },
            ),
        )


def test_append_data_with_invalid_special_column():
    with Bag(name="test_bag_with_invalid") as bag:
        # Append data
        with pytest.raises(ValueError):
            bag.append_data(
                "python-table-data",
                DataFrameData.from_pandas(
                    test_df_with_invalid,
                    {
                        "c": SpecialColumn.Rectangle,
                        "d": SpecialColumn.Segment,
                    },
                ),
            )


def test_append_tensor2d_data():
    with Bag(name="test_bag_with_tensor2d") as bag:
        # Append data

        gauss = np.fromfunction(lambda y, x: np.exp(-((x - 256.0) / 100.0) ** 2 - ((y - 256.0) / 100.0) ** 2), (512, 512), dtype=np.float32)

        print(gauss)

        bag.append_data(
            "python-tensor2d-data",
            Tensor2DData.from_numpy(
                gauss
            ),
        )


def test_append_color_data():
    with Bag(name="test_bag_with_color") as bag:
        # Append data
        bag.append_data(
            "python-color-data",
            DataFrameData.from_pandas(
                test_df_with_color,
                {
                    "color": SpecialColumn.Color,
                    "rect": SpecialColumn.Rectangle,
                },
            ),
        )


@pytest.mark.skip(reason="まだrustのライブラリをこちらに持って来れていないため")
def test_dataframe_encode_and_decode():
    df = pandas.DataFrame(
        [
            {
                "a": 1,
                "b": 2,
                "c": Rectangle(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump(),
            },
            {
                "a": 3,
                "b": 4,
                "c": Rectangle(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump(),
            },
            {
                "a": 5,
                "b": 6,
                "c": Rectangle(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump(),
            },
        ]
    )
    pa_df = pyarrow.Table.from_pandas(df)
    print(pa_df)
    sink = BytesIO()
    with pyarrow.ipc.new_file(sink, pa_df.schema) as writer:
        writer.write(pa_df)

    print(len(sink.getvalue()))

    ret = subprocess.run(["cargo", "run", "--example", "decode"], input=sink.getvalue())

    assert ret.returncode == 0
