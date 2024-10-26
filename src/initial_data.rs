use flexim_data_type::{
    FlDataFrame, FlDataFrameColor, FlDataFrameCubicBezier, FlDataFrameRectangle,
    FlDataFrameSpecialColumn, FlImage, FlObject, FlTensor2D,
};
use flexim_storage::{BagId, Storage};
use ndarray::Array2;
use polars::datatypes::StructChunked;
use polars::prelude::{CsvReadOptions, IntoSeries, NamedFrom, SerReader, Series};
use serde_json::Value;
use std::io::Cursor;
use std::sync::Arc;

pub fn initial_data(storage: Arc<Storage>) -> BagId {
    let bag_id = storage.create_bag("test".to_string());
    storage
        .insert_data(
            bag_id,
            "logo".to_string(),
            FlImage::new(
                include_bytes!("../assets/flexim-logo-1.png").to_vec(),
                512,
                512,
            )
            .into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "tall".to_string(),
            FlImage::new(include_bytes!("../assets/tall.png").to_vec(), 512, 1024).into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "tall".to_string(),
            FlImage::new(include_bytes!("../assets/tall.png").to_vec(), 512, 1024).into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "gauss".to_string(),
            FlTensor2D::new(
                Array2::from_shape_fn((512, 512), |(y, x)| {
                    // center peak gauss
                    let x = (x as f64 - 256.0) / 100.0;
                    let y = (y as f64 - 256.0) / 100.0;
                    (-(x * x + y * y) / 2.0).exp()
                }),
                (0, 0),
            )
            .into(),
        )
        .unwrap();
    storage
        .insert_data(bag_id, "tabledata".to_string(), load_sample_data().into())
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "long_tabledata".to_string(),
            load_long_sample_data().into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "sample_object".to_string(),
            load_object_sample_data().into(),
        )
        .unwrap();

    let _ = storage.create_bag("test2test2test2test2test2".to_string());
    let bag_id = storage.create_bag("test2test2test2test2test2".to_string());
    storage
        .insert_data(
            bag_id,
            "logo".to_string(),
            FlImage::new(include_bytes!("../assets/tall.png").to_vec(), 512, 1442).into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "tall".to_string(),
            FlImage::new(
                include_bytes!("../assets/flexim-logo-1.png").to_vec(),
                512,
                512,
            )
            .into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "tall".to_string(),
            FlImage::new(
                include_bytes!("../assets/flexim-logo-1.png").to_vec(),
                512,
                512,
            )
            .into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "gauss".to_string(),
            FlTensor2D::new(
                Array2::from_shape_fn((512, 512), |(y, x)| {
                    // center peak gauss
                    let x = (x as f64 - 256.0) / 100.0;
                    let y = (y as f64 - 256.0) / 100.0;
                    (-(x * x + y * y) / 2.0).exp()
                }),
                (100, 100),
            )
            .into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "tabledata".to_string(),
            load_long_sample_data2().into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "long_tabledata".to_string(),
            load_long_sample_data().into(),
        )
        .unwrap();

    let _ = storage.create_bag("group/bag1".to_string());
    let _ = storage.create_bag("group/bag2".to_string());
    let _ = storage.create_bag("group/bag2".to_string());

    bag_id
}

fn load_long_sample_data() -> FlDataFrame {
    let data = Vec::from(include_bytes!("../assets/long_sample.csv"));
    let data = Cursor::new(data);
    // let mut df = CsvReader::new(data).has_header(true).finish().unwrap();
    let mut df = CsvReadOptions::default()
        .with_has_header(true)
        .into_reader_with_file_handle(data)
        .finish()
        .unwrap();

    let mut df = df
        .apply("Face", |s| read_rectangle(s, "Face"))
        .unwrap()
        .clone();

    let mut df = df
        .apply("Segment", |s| read_segment(s, "Segment"))
        .unwrap()
        .clone();

    FlDataFrame::new(
        df,
        [
            ("Face".to_string(), FlDataFrameSpecialColumn::Rectangle),
            ("Segment".to_string(), FlDataFrameSpecialColumn::Segment),
        ]
        .into_iter()
        .collect(),
    )
}

fn load_long_sample_data2() -> FlDataFrame {
    let data = Vec::from(include_bytes!("../assets/long_sample2.csv"));
    let data = Cursor::new(data);

    let mut df = CsvReadOptions::default()
        .with_has_header(true)
        .into_reader_with_file_handle(data)
        .finish()
        .unwrap();

    let mut df = df
        .apply("Face1", |s| read_rectangle(s, "Face"))
        .unwrap()
        .clone();

    let df = df
        .apply("Segment1", |s| read_segment(s, "Segment"))
        .unwrap()
        .clone();

    FlDataFrame::new(
        df,
        [
            ("Face1".to_string(), FlDataFrameSpecialColumn::Rectangle),
            ("Segment1".to_string(), FlDataFrameSpecialColumn::Segment),
        ]
        .into_iter()
        .collect(),
    )
}

fn load_object_sample_data() -> FlObject {
    let data = Vec::from(include_bytes!("../assets/object_sample.json"));

    let data: Value = serde_json::from_slice(&data).unwrap();

    FlObject::new(data)
}

fn load_sample_data() -> FlDataFrame {
    let data = Vec::from(include_bytes!("../assets/sample.csv"));
    let data = Cursor::new(data);
    // let mut df = CsvReader::new(data).with_has_header(true).finish().unwrap();

    let mut df = CsvReadOptions::default()
        .with_has_header(true)
        .into_reader_with_file_handle(data)
        .finish()
        .unwrap();

    let mut df = df
        .apply("Face", |s| read_rectangle(s, "Face"))
        .unwrap()
        .clone();

    let mut df = df
        .apply("Segment", |s| read_segment(s, "Segment"))
        .unwrap()
        .clone();

    let mut df = df
        .apply("Color", |s| read_color(s, "Color"))
        .unwrap()
        .clone();

    let df = df
        .apply("Curve", |s| read_curve(s, "Curve"))
        .unwrap()
        .clone();

    FlDataFrame::new(
        df,
        [
            ("Face".to_string(), FlDataFrameSpecialColumn::Rectangle),
            ("Segment".to_string(), FlDataFrameSpecialColumn::Segment),
            ("Color".to_string(), FlDataFrameSpecialColumn::Color),
            ("Curve".to_string(), FlDataFrameSpecialColumn::Curve),
        ]
        .into_iter()
        .collect(),
    )
}

fn read_rectangle(s: &Series, name: &str) -> Series {
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
    let x1 = Series::new("x1", x1);
    let y1 = Series::new("y1", y1);
    let x2 = Series::new("x2", x2);
    let y2 = Series::new("y2", y2);

    StructChunked::new(name, &[x1, y1, x2, y2])
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
    let x1 = Series::new("x1", x1);
    let y1 = Series::new("y1", y1);
    let x2 = Series::new("x2", x2);
    let y2 = Series::new("y2", y2);

    StructChunked::new(name, &[x1, y1, x2, y2])
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
    let r = Series::new("r", r);
    let g = Series::new("g", g);
    let b = Series::new("b", b);

    StructChunked::new(name, &[r, g, b]).unwrap().into_series()
}

fn read_curve(s: &Series, name: &str) -> Series {
    let mut position_x1 = vec![];
    let mut position_y1 = vec![];
    let mut control_x1 = vec![];
    let mut control_y1 = vec![];
    let mut control_x2 = vec![];
    let mut control_y2 = vec![];
    let mut position_x2 = vec![];
    let mut position_y2 = vec![];

    for s in s.str().unwrap().into_iter() {
        let s: Option<&str> = s;
        if let Some(s) = s {
            let t = serde_json::from_str::<FlDataFrameCubicBezier>(s).unwrap();
            position_x1.push(Some(t.position_x1));
            position_y1.push(Some(t.position_y1));
            control_x1.push(Some(t.control_x1));
            control_y1.push(Some(t.control_y1));
            control_x2.push(Some(t.control_x2));
            control_y2.push(Some(t.control_y2));
            position_x2.push(Some(t.position_x2));
            position_y2.push(Some(t.position_y2));
        } else {
            position_x1.push(None);
            position_y1.push(None);
            control_x1.push(None);
            control_y1.push(None);
            control_x2.push(None);
            control_y2.push(None);
            position_x2.push(None);
            position_y2.push(None);
        }
    }

    StructChunked::new(
        name,
        &[
            Series::new("position_x1", position_x1),
            Series::new("position_y1", position_y1),
            Series::new("control_x1", control_x1),
            Series::new("control_y1", control_y1),
            Series::new("control_x2", control_x2),
            Series::new("control_y2", control_y2),
            Series::new("position_x2", position_x2),
            Series::new("position_y2", position_y2),
        ],
    )
    .unwrap()
    .into_series()
}
