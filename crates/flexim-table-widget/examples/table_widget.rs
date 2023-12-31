use flexim_data_type::FlDataFrameRectangle;
use flexim_table_widget::FlTable;
use polars::datatypes::DataType;
use polars::prelude::*;
use polars::series::Series;
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

fn main() {
    let data = Vec::from(include_bytes!("../assets/input.csv"));
    let data = Cursor::new(data);
    let mut df = CsvReader::new(data).has_header(true).finish().unwrap();
    dbg!(&df);

    let df = df.apply("Face", read_rectangle).unwrap().clone();

    dbg!(&df);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    let df = Arc::new(df);

    eframe::run_simple_native("FlTable Example", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut table = FlTable::new(df.clone());
            table.view(ui);
        });
    })
    .unwrap();
}
