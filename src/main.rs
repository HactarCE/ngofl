use std::f64::consts::TAU;

use colorous::RAINBOW;
use egui::Color32;
use egui_plot::{Line, Plot, Points, Polygon};

const POINT_SIZE: f32 = 12.0;
const HOVERED_POINT_SIZE: f32 = 16.0;
const HOVERED_OUTLINE_WIDTH: f32 = 2.0;
const HOVER_DISTANCE: f32 = 10.0;
const LINE_WIDTH: f32 = 3.0;

fn main() -> eframe::Result {
    let mut n = 7;
    let mut points = vec![];
    eframe::run_ui_native(
        "N-gon Flip Puzzle",
        Default::default(),
        move |ui, _frame| {
            let mut reset = false;
            egui::Panel::top("top_panel").show_inside(ui, |ui| {
                ui.add(egui::Slider::new(&mut n, 3..=21).clamping(egui::SliderClamping::Never));
                if n < 3 {
                    n = 3;
                }
                if points.len() != n {
                    points = (0..n)
                        .map(|i| (TAU * i as f64 / n as f64).sin_cos())
                        .map(|(x, y)| [x, y])
                        .collect();
                }

                reset = ui.button("Reset camera").clicked();
            });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                let mut plot = Plot::new("main")
                    .data_aspect(1.0)
                    .default_y_bounds(-2.0, 2.0)
                    .default_x_bounds(-2.0, 2.0)
                    .show_crosshair(false)
                    .allow_boxed_zoom(false);

                if reset {
                    plot = plot.reset();
                }

                let mut hovered_point = None;
                let r = plot.show(ui, |plot_ui| {
                    // Get hovered point
                    hovered_point = plot_ui.pointer_coordinate().and_then(|hov| {
                        points
                            .iter()
                            .map(|&[x, y]| {
                                egui::pos2(x as f32, y as f32).distance_sq(hov.to_pos2())
                            })
                            .enumerate()
                            .min_by(|(_, a), (_, b)| a.total_cmp(b))
                            .filter(|(_, dist)| {
                                plot_ui.transform().dpos_dvalue_x() as f32 * *dist < HOVER_DISTANCE
                            })
                            .map(|(i, _)| i)
                    });

                    // Draw lines
                    plot_ui.add(
                        Polygon::new("", points.clone())
                            .stroke((LINE_WIDTH, Color32::WHITE))
                            .fill_color(Color32::TRANSPARENT),
                    );

                    if let Some(i) = hovered_point {
                        // Draw reflecting line
                        plot_ui.add(
                            Line::new("", vec![points[(i + n - 1) % n], points[(i + 1) % n]])
                                .stroke((LINE_WIDTH, color(i, n))),
                        );

                        // Draw new edges & new point
                        plot_ui.add(
                            Line::new(
                                "",
                                vec![
                                    points[(i + n - 1) % n],
                                    reflected(&points, i),
                                    points[(i + 1) % n],
                                ],
                            )
                            .stroke((LINE_WIDTH, Color32::WHITE))
                            .style(egui_plot::LineStyle::Dotted { spacing: 24.0 }),
                        );
                        plot_ui.add(
                            Points::new("", reflected(&points, i))
                                .radius(POINT_SIZE)
                                .color(color(i, n).gamma_multiply(0.5)),
                        );
                    }

                    // Draw points
                    for (i, xy) in points.iter_mut().enumerate() {
                        let r = if hovered_point == Some(i) {
                            let r = HOVERED_POINT_SIZE + HOVERED_OUTLINE_WIDTH;
                            plot_ui.add(Points::new("", *xy).radius(r).color(Color32::WHITE));
                            HOVERED_POINT_SIZE
                        } else {
                            POINT_SIZE
                        };
                        plot_ui.add(Points::new(name(i), *xy).radius(r).color(color(i, n)));
                    }
                });

                // Update point
                if let Some(i) = hovered_point
                    && r.response.clicked()
                {
                    reflect(&mut points, i);
                }
            });
        },
    )
}

fn reflect(points: &mut Vec<[f64; 2]>, i: usize) {
    points[i] = reflected(points, i);
}

fn reflected(points: &[[f64; 2]], i: usize) -> [f64; 2] {
    let n = points.len();
    let [ax, ay] = points[(i + n - 1) % n];
    let [bx, by] = points[i];
    let [cx, cy] = points[(i + 1) % n];
    [ax + cx - bx, ay + cy - by]
}

fn color(i: usize, n: usize) -> Color32 {
    let [r, g, b] = RAINBOW.eval_rational(i, n).into_array();
    Color32::from_rgb(r, g, b)
}

fn name(i: usize) -> char {
    ('A' as u8 + i as u8) as char
}
