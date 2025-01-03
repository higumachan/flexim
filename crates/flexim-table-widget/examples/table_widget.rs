use flexim_data_type::{
    FlData, FlDataFrame, FlDataFrameColor, FlDataFrameRectangle, FlDataFrameSpecialColumn,
    FlDataReference, FlDataType, GenerationSelector,
};
use flexim_table_widget::{FlTable, FlTableDrawContext};

use flexim_storage::{Storage, StorageQuery};
use polars::prelude::*;
use polars::series::Series;
use std::collections::HashMap;
use std::io::Cursor;

fn read_rectangle(c: &Column) -> Series {
    let mut x1 = vec![];
    let mut y1 = vec![];
    let mut x2 = vec![];
    let mut y2 = vec![];
    for s in c.as_series().unwrap().str().unwrap().into_iter() {
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
    let x1 = Series::new("x1".into(), x1);
    let y1 = Series::new("y1".into(), y1);
    let x2 = Series::new("x2".into(), x2);
    let y2 = Series::new("y2".into(), y2);

    StructChunked::from_series("Face".into(), x1.len(), [x1, y1, x2, y2].iter())
        .unwrap()
        .into_series()
}

fn read_segment(s: &Series, name: &str) -> Series {
    let mut x1 = vec![];
    let mut y1 = vec![];
    let mut x2 = vec![];
    let mut y2 = vec![];
    for s in s.str().unwrap().into_iter() {
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
    let x1 = Series::new("x1".into(), x1);
    let y1 = Series::new("y1".into(), y1);
    let x2 = Series::new("x2".into(), x2);
    let y2 = Series::new("y2".into(), y2);

    StructChunked::from_series(name.into(), x1.len(), [x1, y1, x2, y2].iter())
        .unwrap()
        .into_series()
}

fn read_color(s: &Series, name: &str) -> Series {
    let mut r = vec![];
    let mut g = vec![];
    let mut b = vec![];
    for s in s.str().unwrap().into_iter() {
        let s: Option<&str> = s;
        if let Some(s) = s {
            let t = serde_json::from_str::<FlDataFrameColor>(s).unwrap();
            r.push(Some(t.r));
            g.push(Some(t.g));
            b.push(Some(t.b));
        } else {
            r.push(None);
            g.push(None);
            b.push(None);
        }
    }
    let r = Series::new("r".into(), r);
    let g = Series::new("g".into(), g);
    let b = Series::new("b".into(), b);

    StructChunked::from_series(name.into(), r.len(), [r, g, b].iter())
        .unwrap()
        .into_series()
}

#[allow(clippy::dbg_macro)]
fn main() {
    let data = Vec::from(include_bytes!("../assets/input.csv"));
    let data = Cursor::new(data);
    // let mut df = CsvReader::new(data).has_header(true).finish().unwrap();
    let mut df = CsvReadOptions::default()
        .with_has_header(true)
        .into_reader_with_file_handle(data)
        .finish()
        .unwrap();

    let mut df = df.apply("Face", read_rectangle).unwrap().clone();
    let mut df = df
        .apply("Segment", |s| {
            read_segment(s.as_series().unwrap(), "Segment")
        })
        .unwrap()
        .clone();
    let df = df
        .apply("Color", |s| read_color(s.as_series().unwrap(), "Color"))
        .unwrap()
        .clone();

    dbg!(&df);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    let mut special_columns = HashMap::new();
    special_columns.insert("Face".to_string(), FlDataFrameSpecialColumn::Rectangle);
    special_columns.insert("Segment".to_string(), FlDataFrameSpecialColumn::Segment);
    special_columns.insert("Color".to_string(), FlDataFrameSpecialColumn::Color);
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
    let bag = storage.get_bag(bag_id).unwrap();

    eframe::run_simple_native("FlTable Example", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let table = FlTable::new(FlDataReference::new(
                "dataframe".to_string(),
                GenerationSelector::Latest,
                FlDataType::DataFrame,
            ));
            let bag = bag.read().unwrap();
            table.draw(ui, &bag, &FlTableDrawContext::default());
        });
    })
    .unwrap();
}
