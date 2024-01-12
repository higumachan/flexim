from PIL import Image

from flexim.bag import Bag
from flexim.client import init
from flexim.data_type import ImageData


def test_simple_append_data():
    # Create a bag
    init(host="localhost", port=50051)
    with Bag(name="test_bag") as bag:
        # Append data
        bag.append_data("python-data", ImageData.from_pil(Image.open("../../assets/flexim-logo-1.png")))
