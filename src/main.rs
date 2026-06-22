use std::f32::consts::{FRAC_PI_2, PI, TAU};

use colorous::RAINBOW;
use egui::{Color32, NumExt};
use egui_plot::{GridMark, Line, Plot, Points, Polygon};

const POINT_SIZE: f32 = 12.0;
const HOVERED_POINT_SIZE: f32 = 16.0;
const HOVERED_OUTLINE_WIDTH: f32 = 2.0;
const HOVER_DISTANCE: f32 = 10.0;
const LINE_WIDTH: f32 = 3.0;

mod shortcuts {
    use egui::{Key, KeyboardShortcut, Modifiers};

    pub const CMD_Z: KeyboardShortcut = KeyboardShortcut {
        modifiers: Modifiers::COMMAND,
        logical_key: Key::Z,
    };
    pub const CMD_SHIFT_Z: KeyboardShortcut = KeyboardShortcut {
        modifiers: Modifiers::COMMAND.plus(Modifiers::SHIFT),
        logical_key: Key::Z,
    };
    pub const CMD_Y: KeyboardShortcut = KeyboardShortcut {
        modifiers: Modifiers::COMMAND,
        logical_key: Key::Y,
    };
    pub const CMD_R: KeyboardShortcut = KeyboardShortcut {
        modifiers: Modifiers::COMMAND,
        logical_key: Key::R,
    };
}

fn main() -> eframe::Result {
    let mut first_frame = true;

    let mut undo_stack = vec![];
    let mut redo_stack = vec![];

    let mut n = 7;
    let mut points = vec![];
    eframe::run_ui_native(
        "N-gon Flip Puzzle",
        Default::default(),
        move |ui, _frame| {
            if std::mem::take(&mut first_frame) {
                ui.ctx().set_zoom_factor(1.5);
            }

            ui.input_mut(|input| {
                if input.consume_shortcut(&shortcuts::CMD_R) {
                    points.clear();
                } else if input.consume_shortcut(&shortcuts::CMD_SHIFT_Z)
                    || input.consume_shortcut(&shortcuts::CMD_Y)
                {
                    if let Some(i) = redo_stack.pop() {
                        reflect(&mut points, i);
                        undo_stack.push(i);
                    }
                } else if input.consume_shortcut(&shortcuts::CMD_Z) {
                    if let Some(i) = undo_stack.pop() {
                        reflect(&mut points, i);
                        redo_stack.push(i);
                    }
                }
            });

            let mut reset_view = false;
            egui::Panel::bottom("bottom_panel").show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    let input_button = egui::containers::menu::MenuButton::new("Input").config(
                        egui::containers::menu::MenuConfig::new()
                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                    );
                    input_button.ui(ui, |ui| {
                        let size = ui.content_rect().size().min_elem() * 0.25;
                        let (response, painter) =
                            ui.allocate_painter(egui::vec2(size, size), egui::Sense::click());
                        let rect = response.rect;
                        let c = rect.center();
                        let r = rect.width() * 0.5;
                        let r1 = r / 3.0;
                        let r2 = r;
                        let hovered_index = response
                            .hover_pos()
                            .map(|pos| pos - c)
                            .filter(|v| (r1..r2).contains(&v.length()))
                            .map(|v| {
                                (((v.angle() / TAU) + 1.25) * n as f32 + 0.5).floor() as usize % n
                            });
                        for i in 0..n {
                            let angle1 = (TAU + angle(i, n) - PI / n as f32) % TAU;
                            let angle2 = (TAU + angle(i + 1, n) - PI / n as f32) % TAU;
                            let is_hovered = hovered_index == Some(i);
                            let steps = (100 / n).at_least(2);
                            let shape = egui::Shape::convex_polygon(
                                std::iter::chain(
                                    (0..=steps)
                                        .map(|k| k as f32 / steps as f32)
                                        .map(|t| angle1 + t * TAU / n as f32)
                                        .map(|angle| {
                                            c + r1 * egui::Vec2::angled(angle - FRAC_PI_2)
                                        }),
                                    (0..=steps)
                                        .map(|k| k as f32 / steps as f32)
                                        .map(|t| angle2 - t * TAU / n as f32)
                                        .map(|angle| {
                                            c + r2 * egui::Vec2::angled(angle - FRAC_PI_2)
                                        }),
                                )
                                .collect(),
                                color(i, n).gamma_multiply(if is_hovered { 1.0 } else { 0.5 }),
                                egui::Stroke::NONE,
                            );
                            painter.add(shape);

                            if response.clicked() && is_hovered {
                                reflect(&mut points, i);
                                undo_stack.push(i);
                                redo_stack.clear();
                            }
                        }
                    });

                    ui.separator();

                    ui.add(
                        egui::Slider::new(&mut n, 3..=26).clamping(egui::SliderClamping::Always),
                    );

                    if ui.button("Reset points").clicked() {
                        points.clear();
                    }

                    ui.separator();

                    reset_view = ui.button("Reset view").clicked();

                    ui.separator();

                    ui.add(egui::Label::new("History:").selectable(false));
                    let mut s = undo_stack.iter().map(|&i| name(i)).collect::<String>();
                    if ui.add(egui::TextEdit::singleline(&mut s)).changed() {
                        init(&mut points, n);
                        undo_stack.clear();
                        redo_stack.clear();
                        for c in s.chars() {
                            let c = c.to_ascii_uppercase();
                            let i = if c.is_ascii_alphabetic() {
                                Some(c as usize - 'A' as usize)
                            } else if c == '0' {
                                Some(9)
                            } else if c.is_ascii_digit() {
                                Some(c as usize - '1' as usize)
                            } else {
                                None
                            };
                            if let Some(i) = i {
                                reflect(&mut points, i);
                                undo_stack.push(i);
                            }
                        }
                    }
                });
            });

            if points.len() != n {
                init(&mut points, n);
                undo_stack.clear();
                redo_stack.clear();
            }

            egui::CentralPanel::default().show_inside(ui, |ui| {
                let mut plot = Plot::new("main")
                    .data_aspect(1.0)
                    .default_y_bounds(-2.0, 2.0)
                    .default_x_bounds(-2.0, 2.0)
                    .show_crosshair(false)
                    .allow_boxed_zoom(false)
                    .show_axes(false)
                    .x_grid_spacer(|_| {
                        vec![GridMark {
                            value: 0.0,
                            step_size: 1.0,
                        }]
                    })
                    .y_grid_spacer(|_| {
                        vec![GridMark {
                            value: 0.0,
                            step_size: 1.0,
                        }]
                    });

                if reset_view {
                    plot = plot.reset();
                    undo_stack.clear();
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
                            .stroke((LINE_WIDTH / 2.0, Color32::WHITE))
                            .style(egui_plot::LineStyle::Dotted { spacing: 8.0 }),
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
                    undo_stack.push(i);
                    redo_stack.clear();
                }
            });
        },
    )
}

fn init(points: &mut Vec<[f64; 2]>, n: usize) {
    *points = (0..n)
        .map(|i| (angle(i, n) as f64).sin_cos())
        .map(|(x, y)| [x, y])
        .collect();
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

fn angle(i: usize, n: usize) -> f32 {
    (i as f32 / n as f32) * TAU
}

fn name(i: usize) -> char {
    ('A' as u8 + i as u8) as char
}
