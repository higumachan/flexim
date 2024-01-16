use egui_extras::install_image_loaders;
use flexim_data_type::{FlImage, FlTensor2D};
use flexim_data_visualize::visualize::{
    stack_visualize, DataRender, FlImageRender, FlTensor2DRender, VisualizeState,
};
use ndarray::Array2;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let mut state = VisualizeState::default();
    let stack: Vec<Arc<dyn DataRender>> = vec![
        Arc::new(FlImageRender::new(Arc::new(
            FlImage::new(
                include_bytes!("../../../assets/flexim-logo-1.png").to_vec(),
                512,
                512,
            )
            .into(),
        ))),
        Arc::new(FlTensor2DRender::new(Arc::new(
            FlTensor2D::new(Array2::from_shape_fn((512, 512), |(y, x)| {
                // center peak gauss
                let x = (x as f64 - 256.0) / 100.0;
                let y = (y as f64 - 256.0) / 100.0;
                (-(x * x + y * y) / 2.0).exp()
            }))
            .into(),
        ))),
    ];

    eframe::run_simple_native("stack check", options, move |ctx, _frame| {
        install_image_loaders(ctx);

        egui::CentralPanel::default().show(ctx, |ui| stack_visualize(ui, &mut state, &stack));
    })
}
