import subprocess
from io import BytesIO

import pandas
import pyarrow
import pyarrow.ipc
from PIL import Image

from flexim_py.bag import Bag
from flexim_py.client import init
from flexim_py.data_type import ImageData, Rectangle, DataFrameData

test_df = pandas.DataFrame([
    {"a": 1, "b": 2, "c": Rectangle(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump()},
    {"a": 3, "b": 4, "c": Rectangle(x1=100.0, y1=50.0, x2=200.0, y2=300.0).model_dump()},
    {"a": 5, "b": 6, "c": Rectangle(x1=400.0, y1=50.0, x2=100.0, y2=500.0).model_dump()},
])


def test_simple_append_data():
    # Create a bag
    init(host="localhost", port=50051)
    with Bag(name="test_bag") as bag:
        # Append data
        bag.append_data("python-image-data", ImageData.from_pil(Image.open("../assets/flexim-logo-1.png")))
        bag.append_data("python-table-data", DataFrameData.from_pandas(test_df, {}))


def test_dataframe_encode_and_decode():
    df = pandas.DataFrame([
        {"a": 1, "b": 2, "c": Rectangle(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump()},
        {"a": 3, "b": 4, "c": Rectangle(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump()},
        {"a": 5, "b": 6, "c": Rectangle(x1=100.0, y1=100.0, x2=200.0, y2=200.0).model_dump()},
    ])
    pa_df = pyarrow.Table.from_pandas(df)
    print(pa_df)
    sink = BytesIO()
    with pyarrow.ipc.new_file(sink, pa_df.schema) as writer:
        writer.write(pa_df)

    print(len(sink.getvalue()))

    ret = subprocess.run(["cargo", "run", "--example", "decode"], input=sink.getvalue())

    assert ret.returncode