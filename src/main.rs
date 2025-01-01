use std::default::Default;

use eframe::run_native;
use egui::ahash::{HashMap, HashMapExt};

use egui_extras::install_image_loaders;
use egui_tiles::Tree;
use flexim::left_panel::left_panel;
use flexim::App;
use flexim_connect::grpc::flexim_connect_server::FleximConnectServer;
use flexim_connect::server::FleximConnectServerImpl;
use flexim_data_type::{
    FlDataFrame, FlDataFrameColor, FlDataFrameRectangle, FlDataFrameSpecialColumn, FlDataReference,
    FlDataType, FlImage, FlObject, FlTensor2D, GenerationSelector,
};
use flexim_data_visualize::visualize::{DataRender, FlImageRender};
use flexim_font::setup_custom_fonts;
use flexim_layout::pane::{Pane, PaneContent};
use flexim_storage::Storage;
use ndarray::Array2;
use polars::prelude::{
    Column, CsvReadOptions, IntoSeries, NamedFrom, SerReader, Series, StructChunked,
};
use serde_json::Value;
use std::fmt::Debug;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use tonic::transport::Server;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
    let _puffin_server = puffin_http::Server::new(&server_addr);
    eprintln!("Run this to view profiling data:  puffin_viewer {server_addr}");
    puffin::set_scopes_on(true);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default(),
        ..Default::default()
    };

    let storage = Arc::new(Storage::default());
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

    {
        let storage = storage.clone();
        std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let addr = "[::1]:50051".parse().unwrap();
                let server_impl = FleximConnectServerImpl::new(storage);

                Server::builder()
                    .add_service(FleximConnectServer::new(server_impl))
                    .serve(addr)
                    .await
                    .unwrap();
            });
        });
    }

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

    let tree = create_tree();
    let app = App::new(tree, storage, Some(bag_id));

    run_native(
        "Flexim",
        options,
        Box::new(move |cc| {
            setup_custom_fonts(&cc.egui_ctx);
            install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}

fn create_tree() -> Tree<Pane> {
    let mut next_view_nr = 0;
    let mut gen_pane = |name: String, image: Arc<DataRender>| {
        let pane = Pane {
            name,
            content: PaneContent::Visualize(image),
        };
        next_view_nr += 1;
        pane
    };

    let mut tiles = egui_tiles::Tiles::default();

    let mut tabs = vec![];
    tabs.push({
        let image1 = Arc::<DataRender>::new(
            FlImageRender::new(FlDataReference::new(
                "logo".to_string(),
                GenerationSelector::Latest,
                FlDataType::Image,
            ))
            .into(),
        );
        let image2 = Arc::<DataRender>::new(
            FlImageRender::new(FlDataReference::new(
                "tall".to_string(),
                GenerationSelector::Latest,
                FlDataType::Image,
            ))
            .into(),
        );
        let children = vec![
            tiles.insert_pane(gen_pane("image".to_string(), image1.clone())),
            tiles.insert_pane(gen_pane("tall".to_string(), image2.clone())),
        ];

        tiles.insert_horizontal_tile(children)
    });

    let root = tiles.insert_tab_tile(tabs);

    egui_tiles::Tree::new("flexim", root, tiles)
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

    let df = df
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
