use flexim_data_type::{FlDataReference, FlDataType};
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

impl PaneContent {
    pub fn reference(&self) -> FlDataReference {
        match self {
            Self::Visualize(render) => render.reference(),
            Self::DataView(view) => view.reference(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Pane {
    pub name: String,
    pub content: PaneContent,
}

impl Pane {
    pub fn new(name: String, content: PaneContent) -> Self {
        Self { name, content }
    }
}

pub fn into_pane_content(fl_data_reference: FlDataReference) -> anyhow::Result<PaneContent> {
    match fl_data_reference.data_type {
        FlDataType::Image => Ok(PaneContent::Visualize(Arc::new(
            FlImageRender::new(fl_data_reference).into(),
        ))),
        FlDataType::Tensor => Ok(PaneContent::Visualize(Arc::new(
            FlTensor2DRender::new(fl_data_reference).into(),
        ))),
        FlDataType::DataFrame => Ok(PaneContent::DataView(Arc::new(DataView::FlDataFrameView(
            FlDataFrameView::new(fl_data_reference),
        )))),
    }
}
