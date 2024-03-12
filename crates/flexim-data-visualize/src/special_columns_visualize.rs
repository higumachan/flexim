use crate::visualize::VisualizeState;
use egui::epaint::PathShape;
use egui::CursorIcon::Default;
use egui::{
    Align2, Color32, FontId, Painter, Pos2, Rangef, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
};
use flexim_data_type::{FlDataFrameRectangle, FlDataFrameSegment};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

pub trait SpecialColumnShape: Debug {
    fn render(
        &self,
        ui: &mut Ui,
        painter: &mut Painter,
        parameter: RenderParameter,
        state: &VisualizeState,
    ) -> Option<Response>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum EdgeAccent {
    #[default]
    None,
    Arrow,
}

impl Display for EdgeAccent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Arrow => write!(f, "Arrow"),
        }
    }
}

pub struct RenderParameter {
    pub stroke_color: Color32,
    pub stroke_thickness: f32,
    pub fill_color: Option<Color32>,
    pub edge_accent: EdgeAccent,
    pub label: Option<String>,
}

impl SpecialColumnShape for FlDataFrameRectangle {
    fn render(
        &self,
        ui: &mut Ui,
        painter: &mut Painter,
        parameter: RenderParameter,
        state: &VisualizeState,
    ) -> Option<Response> {
        let RenderParameter {
            stroke_color: color,
            stroke_thickness: thickness,
            label,
            fill_color,
            ..
        } = parameter;

        let rect = Rect::from_two_pos(
            painter.clip_rect().min
                + (Vec2::new(self.x1 as f32, self.y1 as f32) * state.scale() + state.shift),
            painter.clip_rect().min
                + (Vec2::new(self.x2 as f32, self.y2 as f32) * state.scale() + state.shift),
        );
        if let Some(fill_color) = fill_color {
            painter.rect_filled(rect, 0.0, fill_color);
        }
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
                label.as_str(),
                FontId::default(),
                Color32::BLACK,
            );
            painter.rect_filled(text_rect, 0.0, color);
            let text_rect = painter.text(
                rect.left_top(),
                Align2::LEFT_BOTTOM,
                label.as_str(),
                FontId::default(),
                Color32::BLACK,
            );
            responses.push(ui.allocate_rect(text_rect, Sense::click()));
        }

        let last = responses.pop()?;
        Some(responses.into_iter().fold(last, |acc, r| acc.union(r)))
    }
}

impl SpecialColumnShape for FlDataFrameSegment {
    fn render(
        &self,
        ui: &mut Ui,
        painter: &mut Painter,
        parameter: RenderParameter,
        state: &VisualizeState,
    ) -> Option<Response> {
        let RenderParameter {
            stroke_color: color,
            stroke_thickness: thickness,
            label,
            edge_accent,
            ..
        } = parameter;

        let segment_p1 = (Vec2::new(self.x1 as f32, self.y1 as f32) * state.scale()).to_pos2()
            + state.shift
            + painter.clip_rect().min.to_vec2();
        let segment_p2 = (Vec2::new(self.x2 as f32, self.y2 as f32) * state.scale()).to_pos2()
            + state.shift
            + painter.clip_rect().min.to_vec2();
        let center = (segment_p1 + segment_p2.to_vec2()) / 2.0;

        let rectangle = Rect::from_min_max(segment_p1, segment_p2);
        let response = if rectangle.width() < 1.0 || rectangle.height() < 1.0 {
            ui.allocate_rect(rectangle, Sense::click())
        } else {
            let p1_rectangle = Rect::from_center_size(segment_p1, Vec2::splat(1.0));
            let p2_rectangle = Rect::from_center_size(segment_p2, Vec2::splat(1.0));
            let center_rectangle = Rect::from_center_size(center, Vec2::splat(1.0));

            ui.allocate_rect(p1_rectangle, Sense::click())
                .union(ui.allocate_rect(p2_rectangle, Sense::click()))
                .union(ui.allocate_rect(center_rectangle, Sense::click()))
        };

        painter.line_segment([segment_p1, segment_p2], Stroke::new(thickness, color));

        match edge_accent {
            EdgeAccent::Arrow => {
                let v = (segment_p1 - segment_p2).normalized();
                painter.add(arrow_head_shape(segment_p2, v, color));
            }
            EdgeAccent::None => {}
        }

        let response = if let Some(label) = label {
            let text_rect = painter.text(
                center,
                Align2::CENTER_CENTER,
                label.as_str(),
                FontId::default(),
                Color32::BLACK,
            );
            painter.rect_filled(text_rect, 0.0, color);
            let text_rect = painter.text(
                center,
                Align2::CENTER_CENTER,
                label.as_str(),
                FontId::default(),
                Color32::BLACK,
            );
            response | (ui.allocate_rect(text_rect, Sense::click()))
        } else {
            response
        };

        Some(response)
    }
}

fn arrow_head_shape(point: Pos2, back_vector: Vec2, fill_color: Color32) -> Shape {
    let v = back_vector.normalized();
    let v1 = Vec2::new(
        v.x * f32::cos(std::f32::consts::FRAC_PI_4) - v.y * f32::sin(std::f32::consts::FRAC_PI_4),
        v.x * f32::sin(std::f32::consts::FRAC_PI_4) + v.y * f32::cos(std::f32::consts::FRAC_PI_4),
    );
    let p1 = point + (v1 * 5.0);
    // rotate -45 degree
    let v2 = Vec2::new(
        v.x * f32::cos(-std::f32::consts::FRAC_PI_4) - v.y * f32::sin(-std::f32::consts::FRAC_PI_4),
        v.x * f32::sin(-std::f32::consts::FRAC_PI_4) + v.y * f32::cos(-std::f32::consts::FRAC_PI_4),
    );
    let p2 = point + (v2 * 5.0);

    Shape::Path(PathShape::convex_polygon(
        vec![point, p2, p1],
        fill_color,
        Stroke::new(0.0, Color32::default()),
    ))
}
