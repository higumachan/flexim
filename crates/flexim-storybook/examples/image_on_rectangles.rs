use flexim_data_type::{
    FlData, FlDataFrame, FlDataFrameRectangle, FlDataFrameSpecialColumn, FlDataReference,
    FlDataType, FlImage, GenerationSelector,
};

use egui::Id;
use egui_extras::install_image_loaders;
use flexim_data_view::FlDataFrameView;
use flexim_data_visualize::visualize::{
    DataRender, FlDataFrameViewRender, FlImageRender, VisualizeState,
};
use flexim_storage::{Storage, StorageQuery};
use flexim_table_widget::FlTable;
use polars::prelude::*;
use polars::series::Series;
use std::collections::HashMap;
use std::io::Cursor;

fn read_rectangle(s: &Series) -> Series {
    let mut x1 = vec![];
    let mut y1 = vec![];
    let mut x2 = vec![];
    let mut y2 = vec![];
    for s in s.utf8().unwrap().into_iter() {
        let s: Option<&str> = s;
        if let Some(s) = s {
            let t = serde_json::from_str::<FlDataFrameRectangle>(s).unwrap();
            x1.push(Some(t.x1));
            y1.push(Some(t.y1));
            x2.push(Some(t.x2));
            y2.push(Some(t.y2));
        } else {
            x1.push(None);
            y1.push(None);
            x2.push(None);
            y2.push(None);
        }
    }
    let x1 = Series::new("x1", x1);
    let y1 = Series::new("y1", y1);
    let x2 = Series::new("x2", x2);
    let y2 = Series::new("y2", y2);

    StructChunked::new("Face", &[x1, y1, x2, y2])
        .unwrap()
        .into_series()
}

#[allow(clippy::dbg_macro)]
fn main() {
    let data = Vec::from(include_bytes!("../../../assets/sample.csv"));
    let data = Cursor::new(data);
    let mut df = CsvReader::new(data).has_header(true).finish().unwrap();

    let df = df.apply("Face", read_rectangle).unwrap().clone();

    dbg!(&df);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    let mut special_columns = HashMap::new();
    special_columns.insert("Face".to_string(), FlDataFrameSpecialColumn::Rectangle);
    let df = Arc::new(FlDataFrame::new(df, special_columns));
    let storage = Storage::default();
    let bag_id = storage.create_bag("test".to_string());
    storage
        .insert_data(
            bag_id,
            "dataframe".to_string(),
            FlData::DataFrame(df.clone()),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "image".to_string(),
            FlData::Image(Arc::new(FlImage::new(
                include_bytes!("../../../assets/flexim-logo-1.png").to_vec(),
                512,
                512,
            ))),
        )
        .unwrap();
    let bag = storage.get_bag(bag_id).unwrap();

    eframe::run_simple_native("FlTable Example", options, move |ctx, _frame| {
        install_image_loaders(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            let table = FlTable::new(FlDataReference::new(
                "dataframe".to_string(),
                GenerationSelector::Latest,
                FlDataType::DataFrame,
            ));
            let bag = bag.read().unwrap();
            let stack = vec![
                Arc::new(DataRender::Image(FlImageRender::new(FlDataReference::new(
                    "image".to_string(),
                    GenerationSelector::Latest,
                    FlDataType::Image,
                )))),
                Arc::new(DataRender::DataFrameView(Box::new(
                    FlDataFrameViewRender::new(
                        FlDataFrameView::new(FlDataReference::new(
                            "dataframe".to_string(),
                            GenerationSelector::Latest,
                            FlDataType::DataFrame,
                        )),
                        "Face".to_string(),
                    ),
                ))),
            ];
            let mut visualize_state = VisualizeState::load(ctx, Id::new("stack"));
            visualize_state.show(ui, &bag, &stack);
        });
    })
    .unwrap();
}
