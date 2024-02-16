import subprocess
from io import BytesIO

import pandas
import pyarrow
import pyarrow.ipc
import pytest
from PIL import Image

from flexim_py.bag import Bag
from flexim_py.client import init
from flexim_py.data_type import (
    ImageData,
    Rectangle,
    DataFrameData,
    Segment,
    SpecialColumn,
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

def test_simple_append_data():
    # Create a bag
    init(host="localhost", port=50051)
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
    init(host="localhost", port=50051)
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
    init(host="localhost", port=50051)
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
