use flexim_data_type::FlData;
use flexim_data_view::FlDataFrameView;
use flexim_data_visualize::data_view::DataView;
use flexim_data_visualize::visualize::{DataRender, FlImageRender, FlTensor2DRender};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize)]
pub enum PaneContent {
    Visualize(Arc<DataRender>),
    DataView(Arc<DataView>),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Pane {
    pub name: String,
    pub content: PaneContent,
}

pub fn into_pane_content(fl_data: &FlData) -> anyhow::Result<PaneContent> {
    match fl_data {
        FlData::Image(fl_image) => Ok(PaneContent::Visualize(Arc::new(
            FlImageRender::new(fl_image.clone()).into(),
        ))),
        FlData::Tensor(fl_tensor2d) => Ok(PaneContent::Visualize(Arc::new(
            FlTensor2DRender::new(fl_tensor2d.clone()).into(),
        ))),
        FlData::DataFrame(fl_dataframe) => Ok(PaneContent::DataView(Arc::new(
            DataView::FlDataFrameView(FlDataFrameView::new(fl_dataframe.clone())),
        ))),
    }
}
