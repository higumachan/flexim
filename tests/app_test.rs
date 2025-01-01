use eframe::epaint::ahash::{HashMap, HashMapExt};
use eframe::App as _;
use egui::accesskit::Role;
use egui::cache::CacheTrait;
use egui::Id;
use egui_extras::install_image_loaders;
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, HarnessBuilder};
use egui_tiles::Tree;
use flexim::App;
use flexim_data_type::{
    FlDataFrame, FlDataFrameColor, FlDataFrameRectangle, FlDataFrameSpecialColumn, FlDataReference,
    FlDataType, GenerationSelector,
};
use flexim_data_view::FlDataFrameView;
use flexim_data_visualize::visualize::{DataRender, FlDataFrameViewRender, FlImageRender};
use flexim_font::setup_custom_fonts;
use flexim_layout::pane::{Pane, PaneContent};
use flexim_storage::Storage;
use flexim_table_widget::cache::FilteredDataFrameCache;
use itertools::Itertools;
use polars::chunked_array::StructChunked;
use polars::prelude::{Column, CsvReadOptions, IntoSeries, NamedFrom, SerReader, Series};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

/// シンプルなデータバッグでtabledataを表示するテスト
#[test]
fn test_open_start_display() {
    let storage = Arc::new(Storage::default());
    let bag_id = storage.create_bag("test".to_string());
    storage
        .insert_data(bag_id, "tabledata".to_string(), load_sample_data().into())
        .unwrap();

    let tree = Tree::empty("flexim");

    let mut app = App::new(tree, storage, Some(bag_id));

    let mut harness = HarnessBuilder::default().build_state(
        |ctx, _state| {
            setup_custom_fonts(&ctx);
            install_image_loaders(&ctx);
            app.show(ctx);
        },
        (),
    );

    harness.run();
    let buttons = harness.get_all_by_label("+").collect_vec();
    assert_eq!(buttons.len(), 2);
    let button = buttons[1];
    button.click();
    harness.run();
    // データが計算されて表示出来るようになるまで待つ
    while harness.ctx.memory_mut(|mem| {
        let cache = mem.caches.cache::<FilteredDataFrameCache>();
        cache.len()
    }) == 0
    {}
    harness.run();
    harness.run();

    let result = harness.try_wgpu_snapshot(&format!("test_open_start_display"));
    assert!(result.is_ok(), "error {:?}", result);
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

    let df = df
        .apply("Color", |s| read_color(s, "Color"))
        .unwrap()
        .clone();

    FlDataFrame::new(
        df,
        [
            ("Face".to_string(), FlDataFrameSpecialColumn::Rectangle),
            ("Segment".to_string(), FlDataFrameSpecialColumn::Segment),
            ("Color".to_string(), FlDataFrameSpecialColumn::Color),
        ]
        .into_iter()
        .collect(),
    )
}

fn read_rectangle(s: &Column, name: &str) -> Series {
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

fn read_segment(s: &Column, name: &str) -> Series {
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

fn read_color(s: &Column, name: &str) -> Series {
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
