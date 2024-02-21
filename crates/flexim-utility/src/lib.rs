use egui::{Align, Layout, Ui};

pub fn left_and_right_layout<Ctx, LR, RR>(
    ui: &mut Ui,
    ctx: &mut Ctx,
    left_content: impl FnOnce(&mut Ctx, &mut Ui) -> LR,
    right_content: impl FnOnce(&mut Ctx, &mut Ui) -> RR,
) {
    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
        right_content(ctx, ui);
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            left_content(ctx, ui);
        });
    });
}
