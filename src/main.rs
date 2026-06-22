use std::f32::consts::{FRAC_PI_2, PI, TAU};

use colorous::RAINBOW;
use egui::{Color32, NumExt};
use egui_plot::{GridMark, Line, Plot, Points, Polygon};

const TITLE: &str = "N-gon Flip Puzzle";
const DEFAULT_ZOOM: f32 = 1.5;
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

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    eframe::run_native(
        TITLE,
        Default::default(),
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(App::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

struct App {
    undo_stack: Vec<usize>,
    redo_stack: Vec<usize>,

    n: usize,
    points: Vec<[f64; 2]>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            undo_stack: vec![],
            redo_stack: vec![],
            n: 7,
            points: vec![],
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let fg_color = ui.visuals().strong_text_color();

        ui.input_mut(|input| {
            if input.consume_shortcut(&shortcuts::CMD_R) {
                self.points.clear();
            } else if input.consume_shortcut(&shortcuts::CMD_SHIFT_Z)
                || input.consume_shortcut(&shortcuts::CMD_Y)
            {
                if let Some(i) = self.redo_stack.pop() {
                    self.reflect(i);
                    self.undo_stack.push(i);
                }
            } else if input.consume_shortcut(&shortcuts::CMD_Z) {
                if let Some(i) = self.undo_stack.pop() {
                    self.reflect(i);
                    self.redo_stack.push(i);
                }
            }
        });

        egui::Panel::bottom("credits_panel").show_inside(ui, |ui| {
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            ui.spacing_mut().scroll.bar_width /= 1.5;
            ui.spacing_mut().scroll.bar_inner_margin = 0.0;
            let sp = std::mem::take(&mut ui.spacing_mut().item_spacing);
            ui.horizontal(|ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("bottom_bar")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        show_credits(ui);
                        ui.add_space(sp.x);
                        ui.separator();
                        ui.add_space(sp.x);
                        show_powered_by_egui(ui);
                        ui.add_space(sp.x);
                        ui.separator();
                        ui.add_space(sp.x);
                        show_source_code_link(ui);
                    });
            });
        });

        let mut reset_view = false;
        let mut frame = egui::Frame::side_top_panel(ui.style());
        frame.inner_margin.top = 6;
        frame.inner_margin.bottom = 6;
        let bottom_panel = egui::Panel::bottom("bottom_panel").frame(frame);
        bottom_panel.show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                let input_button = egui::containers::menu::MenuButton::new("Input").config(
                    egui::containers::menu::MenuConfig::new()
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                );
                input_button.ui(ui, |ui| {
                    let n = self.n;
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
                                    .map(|angle| c + r1 * egui::Vec2::angled(angle - FRAC_PI_2)),
                                (0..=steps)
                                    .map(|k| k as f32 / steps as f32)
                                    .map(|t| angle2 - t * TAU / n as f32)
                                    .map(|angle| c + r2 * egui::Vec2::angled(angle - FRAC_PI_2)),
                            )
                            .collect(),
                            color(i, n).gamma_multiply(if is_hovered { 1.0 } else { 0.5 }),
                            egui::Stroke::NONE,
                        );
                        painter.add(shape);

                        if response.clicked() && is_hovered {
                            self.do_move(i);
                        }
                    }
                });

                ui.separator();

                ui.add(
                    egui::Slider::new(&mut self.n, 3..=26).clamping(egui::SliderClamping::Always),
                );

                if ui.button("Reset points").clicked() {
                    self.reset();
                    reset_view = true;
                }

                ui.separator();

                reset_view |= ui.button("Reset view").clicked();

                ui.separator();

                ui.add(egui::Label::new("History:").selectable(false));
                let mut s = self.undo_stack.iter().map(|&i| name(i)).collect::<String>();
                if ui.add(egui::TextEdit::singleline(&mut s)).changed() {
                    self.reset();
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
                            self.do_move(i);
                        }
                    }
                }
            });
        });

        if self.points.len() != self.n {
            self.reset();
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
                })
                .allow_scroll(false);

            if reset_view {
                plot = plot.reset();
            }

            let mut hovered_point = None;
            let r = plot.show(ui, |plot_ui| {
                let n = self.n;

                // Get hovered point
                hovered_point = plot_ui.pointer_coordinate().and_then(|hov| {
                    self.points
                        .iter()
                        .map(|&[x, y]| egui::pos2(x as f32, y as f32).distance_sq(hov.to_pos2()))
                        .enumerate()
                        .min_by(|(_, a), (_, b)| a.total_cmp(b))
                        .filter(|(_, dist)| {
                            plot_ui.transform().dpos_dvalue_x() as f32 * *dist < HOVER_DISTANCE
                        })
                        .map(|(i, _)| i)
                });

                // Draw lines
                plot_ui.add(
                    Polygon::new("", self.points.clone())
                        .stroke((LINE_WIDTH, fg_color))
                        .fill_color(Color32::TRANSPARENT),
                );

                if let Some(i) = hovered_point {
                    // Draw reflecting line
                    plot_ui.add(
                        Line::new(
                            "",
                            vec![self.points[(i + n - 1) % n], self.points[(i + 1) % n]],
                        )
                        .stroke((LINE_WIDTH, color(i, n))),
                    );

                    // Draw new edges & new point
                    plot_ui.add(
                        Line::new(
                            "",
                            vec![
                                self.points[(i + n - 1) % n],
                                self.reflected_point(i),
                                self.points[(i + 1) % n],
                            ],
                        )
                        .stroke((LINE_WIDTH / 2.0, fg_color))
                        .style(egui_plot::LineStyle::Dotted { spacing: 8.0 }),
                    );
                    plot_ui.add(
                        Points::new("", self.reflected_point(i))
                            .radius(POINT_SIZE)
                            .color(color(i, n).gamma_multiply(0.5)),
                    );
                }

                // Draw points
                for (i, xy) in self.points.iter_mut().enumerate() {
                    let r = if hovered_point == Some(i) {
                        let r = HOVERED_POINT_SIZE + HOVERED_OUTLINE_WIDTH;
                        plot_ui.add(Points::new("", *xy).radius(r).color(fg_color));
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
                self.do_move(i);
            }
        });
    }
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        cc.egui_ctx.set_zoom_factor(DEFAULT_ZOOM);
        Self::default()
    }

    fn reset(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.points = (0..self.n)
            .map(|i| (angle(i, self.n) as f64).sin_cos())
            .map(|(x, y)| [x, y])
            .collect();
    }

    fn reflect(&mut self, i: usize) {
        self.points[i] = self.reflected_point(i);
    }

    fn do_move(&mut self, i: usize) {
        self.reflect(i);
        self.undo_stack.push(i);
        self.redo_stack.clear();
    }

    fn reflected_point(&self, i: usize) -> [f64; 2] {
        let [ax, ay] = self.points[(i + self.n - 1) % self.n];
        let [bx, by] = self.points[i];
        let [cx, cy] = self.points[(i + 1) % self.n];
        [ax + cx - bx, ay + cy - by]
    }
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

fn show_credits(ui: &mut egui::Ui) {
    ui.label(format!("{TITLE} v{} by ", env!("CARGO_PKG_VERSION")));
    ui.hyperlink_to("Andrew Farkas", "https://ajfarkas.dev/");
}

fn show_powered_by_egui(ui: &mut egui::Ui) {
    ui.label("Powered by ");
    ui.hyperlink_to("egui", "https://github.com/emilk/egui");
    ui.label(" and ");
    ui.hyperlink_to(
        "eframe",
        "https://github.com/emilk/egui/tree/master/crates/eframe",
    );
}

fn show_source_code_link(ui: &mut egui::Ui) {
    ui.hyperlink_to(
        egui::RichText::new(" source code").small(),
        env!("CARGO_PKG_REPOSITORY"),
    );
}
