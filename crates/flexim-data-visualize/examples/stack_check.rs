use egui::Id;
use egui_extras::install_image_loaders;
use flexim_data_type::{
    FlData, FlDataReference, FlDataType, FlImage, FlTensor2D, GenerationSelector,
};
use flexim_data_visualize::visualize::{
    DataRender, FlImageRender, FlTensor2DRender, VisualizeState,
};
use flexim_storage::StorageQuery;
use ndarray::Array2;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let storage = flexim_storage::Storage::default();
    let bag_id = storage.create_bag("test".to_string());
    storage
        .insert_data(
            bag_id,
            "image".to_string(),
            flexim_data_type::FlData::Image(Arc::new(FlImage::new(
                include_bytes!("../../../assets/flexim-logo-1.png").to_vec(),
                512,
                512,
            ))),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "tensor".to_string(),
            FlData::Tensor(Arc::new(FlTensor2D::new(Array2::from_shape_fn(
                (512, 512),
                |(y, x)| {
                    // center peak gauss
                    let x = (x as f64 - 256.0) / 100.0;
                    let y = (y as f64 - 256.0) / 100.0;
                    (-(x * x + y * y) / 2.0).exp()
                },
            )))),
        )
        .unwrap();
    let bag = storage.get_bag(bag_id).unwrap();

    let stack: Vec<Arc<DataRender>> = vec![
        Arc::new(
            FlImageRender::new(FlDataReference::new(
                "image".to_string(),
                GenerationSelector::Latest,
                FlDataType::Image,
            ))
            .into(),
        ),
        Arc::new(
            FlTensor2DRender::new(FlDataReference::new(
                "tensor".to_string(),
                GenerationSelector::Latest,
                FlDataType::Tensor,
            ))
            .into(),
        ),
    ];

    eframe::run_simple_native("stack check", options, move |ctx, _frame| {
        install_image_loaders(ctx);

        let bag = bag.read().unwrap();
        let mut state = VisualizeState::load(ctx, Id::new("stack"));
        egui::CentralPanel::default().show(ctx, |ui| state.show(ui, &bag, &stack));
    })
}
