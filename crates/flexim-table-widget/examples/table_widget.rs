use flexim_data_type::{
    FlData, FlDataFrame, FlDataFrameColor, FlDataFrameRectangle, FlDataFrameSpecialColumn,
    FlDataReference, FlDataType, GenerationSelector,
};
use flexim_table_widget::{FlTable, FlTableDrawContext};

use flexim_storage::{Storage, StorageQuery};
use polars::prelude::*;
use polars::series::Series;
use polars::prelude::Column;
use polars_lazy::dsl::col;
use polars_lazy::frame::IntoLazy;
use polars_lazy::prelude::GetOutput;
use polars::datatypes::{DataType, Field};
use std::collections::HashMap;
use std::io::Cursor;

fn read_rectangle(s: &Series) -> Series {
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

    let df = DataFrame::new(vec![
        Series::new("x1".into(), x1).into(),
        Series::new("y1".into(), y1).into(),
        Series::new("x2".into(), x2).into(),
        Series::new("y2".into(), y2).into(),
    ]).unwrap();
    
    df.into_struct("Rectangle".into()).into_series()
}

fn read_segment(s: &Series, _name: &str) -> Series {
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

    let df = DataFrame::new(vec![
        Series::new("x1".into(), x1).into(),
        Series::new("y1".into(), y1).into(),
        Series::new("x2".into(), x2).into(),
        Series::new("y2".into(), y2).into(),
    ]).unwrap();
    
    df.into_struct("Segment".into()).into_series()
}

fn read_color(s: &Series, _name: &str) -> Series {
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

    let df = DataFrame::new(vec![
        Series::new("r".into(), r).into(),
        Series::new("g".into(), g).into(),
        Series::new("b".into(), b).into(),
    ]).unwrap();
    
    df.into_struct("Color".into()).into_series()
}

#[allow(clippy::dbg_macro)]
fn main() {
    let data = Vec::from(include_bytes!("../assets/input.csv"));
    let data = Cursor::new(data);
    // let mut df = CsvReader::new(data).has_header(true).finish().unwrap();
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .into_reader_with_file_handle(data)
        .finish()
        .unwrap();

    let df = df.lazy().with_column(
        col("Face").map(
            |s| Ok(Some(Column::new("Face".into(), read_rectangle(s.as_series().unwrap())))),
            GetOutput::from_type(DataType::Struct(vec![
                Field::new("x1".into(), DataType::Float64),
                Field::new("y1".into(), DataType::Float64),
                Field::new("x2".into(), DataType::Float64),
                Field::new("y2".into(), DataType::Float64),
            ]))
        ).alias("Face")
    ).collect().unwrap();
    let df = df.lazy().with_column(
        col("Segment").map(
            |s| Ok(Some(Column::new("Segment".into(), read_segment(s.as_series().unwrap(), "Segment")))),
            GetOutput::from_type(DataType::Struct(vec![
                Field::new("x1".into(), DataType::Float64),
                Field::new("y1".into(), DataType::Float64),
                Field::new("x2".into(), DataType::Float64),
                Field::new("y2".into(), DataType::Float64),
            ]))
        ).alias("Segment")
    ).collect().unwrap();
    let df = df.lazy().with_column(
        col("Color").map(
            |s| Ok(Some(Column::new("Color".into(), read_color(s.as_series().unwrap(), "Color")))),
            GetOutput::from_type(DataType::Struct(vec![
                Field::new("r".into(), DataType::Float64),
                Field::new("g".into(), DataType::Float64),
                Field::new("b".into(), DataType::Float64),
            ]))
        ).alias("Color")
    ).collect().unwrap();

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
