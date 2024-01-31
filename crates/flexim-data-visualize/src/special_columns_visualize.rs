use crate::visualize::VisualizeState;
use egui::{
    Align2, Color32, FontId, Painter, Pos2, Rangef, Rect, Response, Sense, Stroke, Ui, Vec2,
};
use flexim_data_type::{FlDataFrameRectangle, FlDataFrameSegment};

pub trait SpecialColumnShape {
    fn render(
        &self,
        ui: &mut Ui,
        painter: &mut Painter,
        color: Color32,
        thickness: f32,
        label: Option<&str>,
        state: &VisualizeState,
    ) -> Vec<Response>;
}

impl SpecialColumnShape for FlDataFrameRectangle {
    fn render(
        &self,
        ui: &mut Ui,
        painter: &mut Painter,
        color: Color32,
        thickness: f32,
        label: Option<&str>,
        state: &VisualizeState,
    ) -> Vec<Response> {
        let rect = Rect::from_min_max(
            painter.clip_rect().min
                + (Vec2::new(self.x1 as f32, self.y1 as f32) * state.scale as f32 + state.shift),
            painter.clip_rect().min
                + (Vec2::new(self.x2 as f32, self.y2 as f32) * state.scale as f32 + state.shift),
        );
        painter.rect_stroke(rect, 0.0, Stroke::new(thickness, color));

        let mut responses = vec![
            ui.allocate_rect(
                Rect::from_x_y_ranges(
                    rect.x_range().expand(thickness),
                    Rangef::point(rect.top()).expand(thickness),
                ),
                Sense::click(),
            ),
            ui.allocate_rect(
                Rect::from_x_y_ranges(
                    rect.x_range().expand(thickness),
                    Rangef::point(rect.bottom()).expand(thickness),
                ),
                Sense::click(),
            ),
            ui.allocate_rect(
                Rect::from_x_y_ranges(
                    Rangef::point(rect.left()).expand(thickness),
                    rect.y_range().expand(thickness),
                ),
                Sense::click(),
            ),
            ui.allocate_rect(
                Rect::from_x_y_ranges(
                    Rangef::point(rect.right()).expand(thickness),
                    rect.y_range().expand(thickness),
                ),
                Sense::click(),
            ),
        ];

        if let Some(label) = label {
            let text_rect = painter.text(
                rect.left_top(),
                Align2::LEFT_BOTTOM,
                label,
                FontId::default(),
                Color32::BLACK,
            );
            painter.rect_filled(text_rect, 0.0, color);
            let text_rect = painter.text(
                rect.left_top(),
                Align2::LEFT_BOTTOM,
                label,
                FontId::default(),
                Color32::BLACK,
            );
            responses.push(ui.allocate_rect(text_rect, Sense::click()));
        }

        responses
    }
}

impl SpecialColumnShape for FlDataFrameSegment {
    fn render(
        &self,
        _ui: &mut Ui,
        painter: &mut Painter,
        color: Color32,
        thickness: f32,
        _label: Option<&str>,
        state: &VisualizeState,
    ) -> Vec<Response> {
        let segmment_p1 = Pos2::new(self.x1 as f32, self.y1 as f32) * state.scale as f32
            + state.shift
            + painter.clip_rect().min.to_vec2();
        let segmment_p2 = Pos2::new(self.x2 as f32, self.y2 as f32) * state.scale as f32
            + state.shift
            + painter.clip_rect().min.to_vec2();
        painter.line_segment([segmment_p1, segmment_p2], Stroke::new(thickness, color));
        vec![]
    }
}
