use crate::visualize::VisualizeState;
use egui::epaint::{CubicBezierShape, PathShape, QuadraticBezierShape, StrokeKind};
use egui::{
    Align2, Color32, FontId, Painter, Pos2, Rangef, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
};
use enum_iterator::Sequence;
use flexim_config::Config;
use flexim_data_type::{FlDataFrameCubicBezier, FlDataFrameQuadraticBezier, FlDataFrameRectangle, FlDataFrameSegment};
use geo::Line;
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

    fn measure_segments(&self) -> Vec<Line>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, Sequence)]
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
    pub edge_accent_start: EdgeAccent,
    pub edge_accent_end: EdgeAccent,
    pub label: Option<String>,
    pub show_bezier_control_points: bool,
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
                + state.absolute_to_screen(Vec2::new(self.x1 as f32, self.y1 as f32)),
            painter.clip_rect().min
                + state.absolute_to_screen(Vec2::new(self.x2 as f32, self.y2 as f32)),
        );
        if let Some(fill_color) = fill_color {
            painter.rect_filled(rect, 0.0, fill_color);
        }
        painter.rect_stroke(rect, 0.0, Stroke::new(thickness, color), StrokeKind::Outside);

        let is_command = ui.input(|input| input.modifiers.command_only());
        let sense = if is_command {
            Sense::hover()
        } else {
            Sense::click()
        };

        let mut responses = vec![
            ui.allocate_rect(
                Rect::from_x_y_ranges(
                    rect.x_range().expand(thickness),
                    Rangef::point(rect.top()).expand(thickness),
                ),
                sense,
            ),
            ui.allocate_rect(
                Rect::from_x_y_ranges(
                    rect.x_range().expand(thickness),
                    Rangef::point(rect.bottom()).expand(thickness),
                ),
                sense,
            ),
            ui.allocate_rect(
                Rect::from_x_y_ranges(
                    Rangef::point(rect.left()).expand(thickness),
                    rect.y_range().expand(thickness),
                ),
                sense,
            ),
            ui.allocate_rect(
                Rect::from_x_y_ranges(
                    Rangef::point(rect.right()).expand(thickness),
                    rect.y_range().expand(thickness),
                ),
                sense,
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

    fn measure_segments(&self) -> Vec<Line> {
        // 4辺を表すVec<Line>を返す
        vec![
            Line::new([self.x1, self.y1], [self.x2, self.y1]),
            Line::new([self.x2, self.y1], [self.x2, self.y2]),
            Line::new([self.x2, self.y2], [self.x1, self.y2]),
            Line::new([self.x1, self.y2], [self.x1, self.y1]),
        ]
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
            edge_accent_start,
            edge_accent_end,
            ..
        } = parameter;

        let mut segment_p1 = (Vec2::new(self.x1 as f32, self.y1 as f32) * state.scale()).to_pos2()
            + state.shift
            + painter.clip_rect().min.to_vec2();
        let mut segment_p2 = (Vec2::new(self.x2 as f32, self.y2 as f32) * state.scale()).to_pos2()
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

        match edge_accent_start {
            EdgeAccent::Arrow => {
                let v = (segment_p2 - segment_p1).normalized();
                let (arrow_shape, arrow_offset) = arrow_head_shape(segment_p1, v, thickness, color);
                painter.add(arrow_shape);
                segment_p1 += arrow_offset;
            }
            EdgeAccent::None => {}
        }
        match edge_accent_end {
            EdgeAccent::Arrow => {
                let v = (segment_p1 - segment_p2).normalized();
                let (arrow_shape, arrow_offset) = arrow_head_shape(segment_p2, v, thickness, color);
                painter.add(arrow_shape);
                segment_p2 += arrow_offset;
            }
            EdgeAccent::None => {}
        }

        painter.line_segment([segment_p1, segment_p2], Stroke::new(thickness, color));

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

    fn measure_segments(&self) -> Vec<Line> {
        vec![Line::new([self.x1, self.y1], [self.x2, self.y2])]
    }
}

impl SpecialColumnShape for FlDataFrameQuadraticBezier {
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
            edge_accent_start,
            edge_accent_end,
            show_bezier_control_points,
            ..
        } = parameter;

        // Get configuration for control point size
        let config = Config::get_global(ui);

        // Transform the bezier points to screen space
        let mut p1 = painter.clip_rect().min
            + state.absolute_to_screen(Vec2::new(self.x1 as f32, self.y1 as f32));
        let control = painter.clip_rect().min
            + state.absolute_to_screen(Vec2::new(self.cx as f32, self.cy as f32));
        let mut p2 = painter.clip_rect().min
            + state.absolute_to_screen(Vec2::new(self.x2 as f32, self.y2 as f32));

        // Handle edge accents (arrows)
        match edge_accent_start {
            EdgeAccent::Arrow => {
                // Calculate tangent at start point (direction from p1 towards control)
                let v = (control - p1).normalized();
                let (arrow_shape, arrow_offset) = arrow_head_shape(p1, v, thickness, color);
                painter.add(arrow_shape);
                p1 += arrow_offset;
            }
            EdgeAccent::None => {}
        }

        match edge_accent_end {
            EdgeAccent::Arrow => {
                // Calculate tangent at end point (direction from p2 towards control)
                let v = (control - p2).normalized();
                let (arrow_shape, arrow_offset) = arrow_head_shape(p2, v, thickness, color);
                painter.add(arrow_shape);
                p2 += arrow_offset;
            }
            EdgeAccent::None => {}
        }

        let points = [p1, control, p2];

        let shape = QuadraticBezierShape {
            points,
            closed: false,
            fill: fill_color.unwrap_or(Color32::TRANSPARENT),
            stroke: Stroke::new(thickness, color).into(),
        };

        painter.add(Shape::QuadraticBezier(shape));

        // Draw control points if enabled
        if show_bezier_control_points {
            let control_color = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 128);

            // Draw control point
            painter.circle_filled(control, config.bezier_control_point_size, control_color);

            // Draw lines from endpoints to control point
            painter.line_segment([p1, control], Stroke::new(1.0, control_color));
            painter.line_segment([p2, control], Stroke::new(1.0, control_color));
        }

        // Create response for interaction
        let bounding_rect = Rect::from_min_max(
            Pos2::new(
                points.iter().map(|p| p.x).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
                points.iter().map(|p| p.y).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
            ),
            Pos2::new(
                points.iter().map(|p| p.x).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
                points.iter().map(|p| p.y).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
            ),
        );

        let is_command = ui.input(|input| input.modifiers.command_only());
        let sense = if is_command {
            Sense::hover()
        } else {
            Sense::click()
        };

        let mut response = ui.allocate_rect(bounding_rect, sense);

        if let Some(label) = label {
            let center = (p1 + p2.to_vec2()) / 2.0;
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
            response = response.union(ui.allocate_rect(text_rect, Sense::click()));
        }

        Some(response)
    }

    fn measure_segments(&self) -> Vec<Line> {
        // For simplicity, return a line from start to end
        // In future, could approximate the curve with multiple segments
        vec![Line::new([self.x1, self.y1], [self.x2, self.y2])]
    }
}

impl SpecialColumnShape for FlDataFrameCubicBezier {
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
            edge_accent_start,
            edge_accent_end,
            show_bezier_control_points,
            ..
        } = parameter;

        // Get configuration for control point size
        let config = Config::get_global(ui);

        // Transform the bezier points to screen space
        let mut p1 = painter.clip_rect().min
            + state.absolute_to_screen(Vec2::new(self.x1 as f32, self.y1 as f32));
        let control1 = painter.clip_rect().min
            + state.absolute_to_screen(Vec2::new(self.c1x as f32, self.c1y as f32));
        let control2 = painter.clip_rect().min
            + state.absolute_to_screen(Vec2::new(self.c2x as f32, self.c2y as f32));
        let mut p2 = painter.clip_rect().min
            + state.absolute_to_screen(Vec2::new(self.x2 as f32, self.y2 as f32));

        // Handle edge accents (arrows)
        match edge_accent_start {
            EdgeAccent::Arrow => {
                // Calculate tangent at start point (direction from p1 towards control1)
                let v = (control1 - p1).normalized();
                let (arrow_shape, arrow_offset) = arrow_head_shape(p1, v, thickness, color);
                painter.add(arrow_shape);
                p1 += arrow_offset;
            }
            EdgeAccent::None => {}
        }

        match edge_accent_end {
            EdgeAccent::Arrow => {
                // Calculate tangent at end point (direction from p2 towards control2)
                let v = (control2 - p2).normalized();
                let (arrow_shape, arrow_offset) = arrow_head_shape(p2, v, thickness, color);
                painter.add(arrow_shape);
                p2 += arrow_offset;
            }
            EdgeAccent::None => {}
        }

        let points = [p1, control1, control2, p2];

        let shape = CubicBezierShape {
            points,
            closed: false,
            fill: fill_color.unwrap_or(Color32::TRANSPARENT),
            stroke: Stroke::new(thickness, color).into(),
        };

        painter.add(Shape::CubicBezier(shape));

        // Draw control points if enabled
        if show_bezier_control_points {
            let control_color = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 128);

            // Draw control points
            painter.circle_filled(control1, config.bezier_control_point_size, control_color);
            painter.circle_filled(control2, config.bezier_control_point_size, control_color);

            // Draw lines from endpoints to control points
            painter.line_segment([p1, control1], Stroke::new(1.0, control_color));
            painter.line_segment([control2, p2], Stroke::new(1.0, control_color));
            // Optional: Draw line between control points
            painter.line_segment([control1, control2], Stroke::new(0.5, control_color));
        }

        // Create response for interaction
        let bounding_rect = Rect::from_min_max(
            Pos2::new(
                points.iter().map(|p| p.x).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
                points.iter().map(|p| p.y).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
            ),
            Pos2::new(
                points.iter().map(|p| p.x).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
                points.iter().map(|p| p.y).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
            ),
        );

        let is_command = ui.input(|input| input.modifiers.command_only());
        let sense = if is_command {
            Sense::hover()
        } else {
            Sense::click()
        };

        let mut response = ui.allocate_rect(bounding_rect, sense);

        if let Some(label) = label {
            let center = (p1 + p2.to_vec2()) / 2.0;
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
            response = response.union(ui.allocate_rect(text_rect, Sense::click()));
        }

        Some(response)
    }

    fn measure_segments(&self) -> Vec<Line> {
        // For simplicity, return a line from start to end
        // In future, could approximate the curve with multiple segments
        vec![Line::new([self.x1, self.y1], [self.x2, self.y2])]
    }
}

fn arrow_head_shape(
    point: Pos2,
    back_vector: Vec2,
    thickness: f32,
    fill_color: Color32,
) -> (Shape, Vec2) {
    let v = back_vector.normalized();
    let v1 = Vec2::new(
        v.x * f32::cos(std::f32::consts::FRAC_PI_4) - v.y * f32::sin(std::f32::consts::FRAC_PI_4),
        v.x * f32::sin(std::f32::consts::FRAC_PI_4) + v.y * f32::cos(std::f32::consts::FRAC_PI_4),
    );
    let p1 = point + (v1 * 5.0 * thickness);
    // rotate -45 degree
    let v2 = Vec2::new(
        v.x * f32::cos(-std::f32::consts::FRAC_PI_4) - v.y * f32::sin(-std::f32::consts::FRAC_PI_4),
        v.x * f32::sin(-std::f32::consts::FRAC_PI_4) + v.y * f32::cos(-std::f32::consts::FRAC_PI_4),
    );
    let p2 = point + (v2 * 5.0 * thickness);

    (
        Shape::Path(PathShape::convex_polygon(
            vec![point, p2, p1],
            fill_color,
            Stroke::new(0.0, Color32::default()),
        )),
        v * 3.0 * thickness,
    )
}
