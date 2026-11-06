//! The window itself (built with the `gui` feature): it draws the state of `window::Window` and passes on what the person does. Everything that can be decided without a screen is
//! decided there, in code that is tested.

use crate::browse::{self, Browser};
use crate::checks::Level;
use crate::lang::{fill, Lang, Msg};
use crate::shot::{ShotRequest, ShotTheme, PATIENCE_FRAMES};
use crate::timeline::format_time;
use crate::window::{Key, Section, Window, MAX_DRAWN, MAX_RATE};
use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, Pos2, Rect, Stroke, Vec2,
};
use eh3d_parser_gcode::{Dialect, Motion};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// the colours of the theme Forge, the one the other programs of the suite start with: amber on charcoal
const BACKGROUND: Color32 = Color32::from_rgb(0x0f, 0x0d, 0x0b);
const PANEL: Color32 = Color32::from_rgb(0x17, 0x13, 0x10);
const RAISED: Color32 = Color32::from_rgb(0x22, 0x1c, 0x17);
const TEXT: Color32 = Color32::from_rgb(0xf4, 0xec, 0xe0);
const DIM: Color32 = Color32::from_rgb(0x9c, 0x8d, 0x7a);
const ACCENT: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
const OK: Color32 = Color32::from_rgb(0x9b, 0xe3, 0x4a);
const DANGER: Color32 = Color32::from_rgb(0xff, 0x54, 0x70);
/// A rapid move that has been made, and one that has not.
const RAPID_DONE: Color32 = Color32::from_rgb(0x4d, 0xd0, 0xe1);
const RAPID_LATER: Color32 = Color32::from_rgb(0x2a, 0x3a, 0x40);
/// A cutting move that has not been made yet.
const CUT_LATER: Color32 = Color32::from_rgb(0x5a, 0x4a, 0x2a);
const EDGE: Color32 = Color32::from_rgb(0x8a, 0x9b, 0xb0);
const CLAMP: Color32 = Color32::from_rgb(0xff, 0x54, 0x70);

enum Outcome {
    Cancel,
    Pick(PathBuf),
}

/// The window that opens a file.
struct Dialog {
    browser: Browser,
}

struct App {
    win: Window,
    /// The picture mode, while the window is there only to be photographed.
    shot: Option<ShotState>,
    dialog: Option<Dialog>,
    fit_pending: bool,
    /// The place of the work under the pointer.
    cursor: Option<(f64, f64)>,
    /// The move that was shown the last time, to bring the table to it when it changes.
    last_current: usize,
    scroll_pending: bool,
}

impl App {
    fn new(lang: Lang, initial: Option<PathBuf>) -> App {
        let mut win = Window::new(lang);
        let mut fit_pending = false;
        if let Some(p) = initial {
            fit_pending = win.open_program(&p);
        }
        App {
            win,
            shot: None,
            dialog: None,
            fit_pending,
            cursor: None,
            last_current: 0,
            scroll_pending: true,
        }
    }

    fn folder(&self) -> PathBuf {
        self.win.work_folder().unwrap_or_else(browse::start_dir)
    }

    /// Opens the window of files: the list of programs to open.
    fn ask(&mut self) {
        let extensions: &[&'static str] = &["nc", "ngc", "gcode", "gc", "tap", "cnc", "txt"];
        self.dialog = Some(Dialog {
            browser: Browser::new(self.folder(), extensions),
        });
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::Panel::top("top").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("EH3D-CAM-DEBUGGER")
                        .strong()
                        .size(18.0)
                        .color(ACCENT),
                );
                ui.separator();
                if ui.button(Msg::OpenProgram.tr(lang)).clicked() {
                    self.ask();
                }
                if ui
                    .add_enabled(
                        self.win.has_work(),
                        egui::Button::new(Msg::Analyse.tr(lang)),
                    )
                    .clicked()
                    && self.win.analyse()
                {
                    self.fit_pending = true;
                }
                if ui
                    .add_enabled(self.win.has_work(), egui::Button::new(Msg::Fit.tr(lang)))
                    .clicked()
                {
                    self.fit_pending = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("language")
                        .selected_text(self.win.lang.name())
                        .show_ui(ui, |ui| {
                            for l in Lang::ALL {
                                ui.selectable_value(&mut self.win.lang, l, l.name());
                            }
                        });
                    ui.label(egui::RichText::new(Msg::Language.tr(lang)).color(DIM));
                });
            });
        });
    }

    /// The title of a group of the panels.
    fn heading(&self, msg: Msg) -> egui::RichText {
        egui::RichText::new(msg.tr(self.win.lang))
            .strong()
            .color(ACCENT)
    }

    /// The settings of one group, each with its name above a box to type in (the clamps in a box of several lines).
    fn fields_of(&mut self, ui: &mut egui::Ui, section: Section) {
        let lang = self.win.lang;
        for i in 0..self.win.fields.len() {
            if self.win.fields[i].section != section {
                continue;
            }
            let label = self.win.fields[i].label.tr(lang);
            let mut text = self.win.fields[i].text.clone();
            ui.label(label);
            let changed = if self.win.fields[i].key == Key::Clamps {
                let edit = egui::TextEdit::multiline(&mut text)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY);
                ui.add(edit).changed()
            } else {
                let edit = egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY);
                ui.add(edit).changed()
            };
            if changed {
                self.win.set_field(i, text);
            }
        }
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("settings")
            .default_size(290.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(4.0);
                    self.program_group(ui);
                    self.machine_group(ui);
                    self.limits_group(ui);
                    self.checks_group(ui);
                    self.findings_group(ui);
                });
            });
    }

    fn program_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(Msg::SectionFile))
            .default_open(true)
            .show(ui, |ui| {
                if ui.button(Msg::OpenProgram.tr(lang)).clicked() {
                    self.ask();
                }
                let shown = self
                    .win
                    .program_info()
                    .unwrap_or_else(|| Msg::NoFile.tr(lang).to_string());
                ui.label(egui::RichText::new(shown).color(DIM));
                for line in self.win.overview_lines() {
                    ui.label(line);
                }
            });
    }

    fn machine_group(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(self.heading(Msg::SectionMachine))
            .default_open(true)
            .show(ui, |ui| self.fields_of(ui, Section::Machine));
    }

    fn limits_group(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(self.heading(Msg::SectionLimits))
            .default_open(false)
            .show(ui, |ui| self.fields_of(ui, Section::Limits));
    }

    fn checks_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(Msg::SectionChecks))
            .default_open(false)
            .show(ui, |ui| {
                ui.label(Msg::Dialect.tr(lang));
                let mut dialect = self.win.dialect;
                let shown = dialect_label(dialect);
                egui::ComboBox::from_id_salt("dialect")
                    .selected_text(shown.tr(lang))
                    .show_ui(ui, |ui| {
                        for d in [Dialect::Grbl, Dialect::Marlin, Dialect::PlanetCnc] {
                            ui.selectable_value(&mut dialect, d, dialect_label(d).tr(lang));
                        }
                    });
                if dialect != self.win.dialect {
                    self.win.set_dialect(dialect);
                }
                let mut laser = self.win.laser;
                if ui.checkbox(&mut laser, Msg::Laser.tr(lang)).changed() {
                    self.win.set_laser(laser);
                }
                let mut strict = self.win.strict;
                if ui.checkbox(&mut strict, Msg::Strict.tr(lang)).changed() {
                    self.win.set_strict(strict);
                }
                self.fields_of(ui, Section::Checks);
            });
    }

    fn findings_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        let Some(a) = &self.win.analysis else {
            return;
        };
        // the findings are copied so that a click can move the window while the list is drawn
        let rows: Vec<(usize, Level, String)> = a
            .findings
            .iter()
            .enumerate()
            .map(|(i, f)| (i, f.level, self.win.finding_text(f)))
            .collect();
        egui::CollapsingHeader::new(self.heading(Msg::SectionFindings))
            .default_open(true)
            .show(ui, |ui| {
                if rows.is_empty() {
                    ui.label(egui::RichText::new(Msg::NoFindings.tr(lang)).color(OK));
                }
                for (i, level, text) in rows {
                    let colour = if level == Level::Alert {
                        DANGER
                    } else {
                        ACCENT
                    };
                    if ui
                        .selectable_label(false, egui::RichText::new(text).color(colour))
                        .clicked()
                    {
                        self.win.go_to_finding(i);
                    }
                }
            });
    }

    /// The panel on the right: the move that is shown and the table of all the moves.
    fn steps_panel(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::Panel::right("steps")
            .default_size(380.0)
            .show(ui, |ui| {
                ui.add_space(4.0);
                ui.label(self.heading(Msg::SectionStep));
                for line in self.win.step_lines() {
                    ui.label(line);
                }
                ui.separator();
                ui.label(self.heading(Msg::SectionSteps));
                let total = self.win.steps();
                if total == 0 {
                    if self.win.has_work() {
                        ui.label(egui::RichText::new(Msg::NoMoves.tr(lang)).color(DIM));
                    }
                    return;
                }
                let row_h = ui.text_style_height(&egui::TextStyle::Body);
                let spacing = ui.spacing().item_spacing.y;
                let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]);
                if self.scroll_pending {
                    let top = (self.win.current as f32 - 3.0).max(0.0) * (row_h + spacing);
                    area = area.vertical_scroll_offset(top);
                    self.scroll_pending = false;
                }
                let mut clicked = None;
                area.show_rows(ui, row_h, total, |ui, range| {
                    let Some(a) = &self.win.analysis else {
                        return;
                    };
                    for i in range {
                        let (Some(s), Some(t)) = (a.trace.segments.get(i), a.timeline.steps.get(i))
                        else {
                            continue;
                        };
                        let kind = match s.motion {
                            Motion::Rapid => "G0",
                            Motion::Linear => "G1",
                            Motion::ArcCw { .. } => "G2",
                            Motion::ArcCcw { .. } => "G3",
                        };
                        let text = format!(
                            "{:>6}  {kind}  {:>9.3} mm  {:>8.2} s  {}",
                            t.line,
                            t.length_mm,
                            t.time_s,
                            format_time(t.start_s)
                        );
                        let colour = if s.motion == Motion::Rapid {
                            RAPID_DONE
                        } else {
                            TEXT
                        };
                        let label = egui::RichText::new(text).monospace().color(colour);
                        if ui.selectable_label(i == self.win.current, label).clicked() {
                            clicked = Some(i);
                        }
                    }
                });
                if let Some(i) = clicked {
                    self.win.set_step(i);
                }
            });
    }

    /// The buttons that go through the moves, and the slider.
    fn controls(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::Panel::bottom("controls").show(ui, |ui| {
            ui.horizontal(|ui| {
                let on = self.win.steps() > 0;
                if ui
                    .add_enabled(on, egui::Button::new(Msg::First.tr(lang)))
                    .clicked()
                {
                    self.win.first();
                }
                if ui
                    .add_enabled(on, egui::Button::new(Msg::Previous.tr(lang)))
                    .clicked()
                {
                    self.win.prev();
                }
                let play = if self.win.playing {
                    Msg::Pause
                } else {
                    Msg::Play
                };
                if ui
                    .add_enabled(on, egui::Button::new(play.tr(lang)))
                    .clicked()
                {
                    self.win.toggle_play();
                }
                if ui
                    .add_enabled(on, egui::Button::new(Msg::Next.tr(lang)))
                    .clicked()
                {
                    self.win.next();
                }
                if ui
                    .add_enabled(on, egui::Button::new(Msg::Last.tr(lang)))
                    .clicked()
                {
                    self.win.last();
                }
                if on {
                    let mut i = self.win.current;
                    let slider = egui::Slider::new(&mut i, 0..=self.win.steps() - 1);
                    if ui.add(slider.show_value(true)).changed() {
                        self.win.set_step(i);
                    }
                    let mut rate = self.win.rate;
                    let speed = egui::Slider::new(&mut rate, 0.1..=MAX_RATE).logarithmic(true);
                    if ui.add(speed.suffix(" /s")).changed() {
                        self.win.set_rate(rate);
                    }
                }
            });
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                let (text, problem) = if self.win.stale {
                    (Msg::Stale.tr(lang).to_string(), true)
                } else {
                    (self.win.status.text(lang), self.win.status.is_problem())
                };
                ui.label(egui::RichText::new(text).color(if problem { DANGER } else { OK }));
                if self.win.truncated {
                    let note = fill(Msg::Truncated.tr(lang), &[&MAX_DRAWN.to_string()]);
                    ui.label(egui::RichText::new(note).color(DIM));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(Msg::ZoomHint.tr(lang)).color(DIM));
                    if let Some((x, y)) = self.cursor {
                        ui.separator();
                        ui.label(egui::RichText::new(self.win.cursor_text(x, y)).color(TEXT));
                    }
                });
            });
        });
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let size = ui.available_size();
            let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
            let rect = response.rect;
            let (w, h) = (f64::from(rect.width()), f64::from(rect.height()));
            painter.rect_filled(rect, 0.0, BACKGROUND);
            self.cursor = None;
            if !self.win.has_work() {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    Msg::DropHint.tr(lang),
                    egui::FontId::proportional(18.0),
                    DIM,
                );
                return;
            }
            if self.fit_pending && w > 40.0 && h > 40.0 {
                self.win.fit(w, h);
                self.fit_pending = false;
            }
            if response.double_clicked() {
                self.fit_pending = true;
            }
            if response.dragged() {
                let d = response.drag_delta();
                self.win.viewport.pan_px(f64::from(d.x), f64::from(d.y));
            }
            if let Some(p) = response.hover_pos() {
                let at = p - rect.min;
                let (sx, sy) = (f64::from(at.x), f64::from(at.y));
                let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
                let factor = (f64::from(scroll) * 0.0015).exp() * f64::from(pinch);
                if (factor - 1.0).abs() > 1e-9 {
                    self.win.viewport.zoom_at(sx, sy, w, h, factor);
                }
                self.cursor = Some(self.win.viewport.to_world(sx, sy, w, h));
            }
            self.paint(&painter, rect);
        });
    }

    fn paint(&self, painter: &egui::Painter, rect: Rect) {
        let vp = self.win.viewport;
        let (w, h) = (f64::from(rect.width()), f64::from(rect.height()));
        let at = |x: f64, y: f64| -> Pos2 {
            let (px, py) = vp.to_screen(x, y, w, h);
            Pos2::new(rect.min.x + px as f32, rect.min.y + py as f32)
        };
        let painter = painter.with_clip_rect(rect);
        let Some(a) = &self.win.analysis else {
            return;
        };
        // the table and the clamps
        if let Some((tw, td)) = a.limits.table {
            let corners = vec![at(0.0, 0.0), at(tw, 0.0), at(tw, td), at(0.0, td)];
            painter.add(egui::Shape::closed_line(corners, Stroke::new(1.2, EDGE)));
        }
        for c in &a.limits.clamps {
            let (p, q) = (at(c.x0, c.y0), at(c.x1, c.y1));
            let r = Rect::from_two_pos(p, q);
            painter.rect_filled(
                r,
                0.0,
                Color32::from_rgba_unmultiplied(0xff, 0x54, 0x70, 60),
            );
            painter.rect_stroke(r, 0.0, Stroke::new(1.2, CLAMP), egui::StrokeKind::Inside);
        }
        // the path: what has been made is bright, what is to come is dim
        let current = self.win.current;
        for p in &self.win.pieces {
            let made = p.step <= current;
            let (colour, width) = match (p.rapid, made) {
                (true, true) => (RAPID_DONE, 1.0),
                (true, false) => (RAPID_LATER, 0.8),
                (false, true) => (ACCENT, 1.8),
                (false, false) => (CUT_LATER, 1.2),
            };
            painter.line_segment([at(p.x0, p.y0), at(p.x1, p.y1)], Stroke::new(width, colour));
        }
        for p in self.win.pieces.iter().filter(|p| p.step == current) {
            painter.line_segment(
                [at(p.x0, p.y0), at(p.x1, p.y1)],
                Stroke::new(3.5, Color32::WHITE),
            );
        }
        // the findings
        for m in &self.win.marks {
            let colour = if m.level == Level::Alert {
                DANGER
            } else {
                ACCENT
            };
            let radius = if m.step == current { 9.0 } else { 6.0 };
            painter.circle_stroke(at(m.x, m.y), radius, Stroke::new(2.0, colour));
        }
        // the head
        if let Some(s) = self.win.segment() {
            painter.circle_filled(at(s.to.x, s.to.y), 5.0, Color32::WHITE);
        }
        self.paint_key(&painter, rect);
    }

    /// The key of the colours, in the corner of the picture.
    fn paint_key(&self, painter: &egui::Painter, rect: Rect) {
        let lang = self.win.lang;
        let mut y = rect.min.y + 12.0;
        let mut line = |colour: Color32, text: &str| {
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(rect.min.x + 12.0, y), Vec2::new(14.0, 14.0)),
                2.0,
                colour,
            );
            painter.text(
                Pos2::new(rect.min.x + 32.0, y + 7.0),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::proportional(13.0),
                TEXT,
            );
            y += 20.0;
        };
        line(ACCENT, Msg::LegendCut.tr(lang));
        line(RAPID_DONE, Msg::LegendRapid.tr(lang));
        line(CUT_LATER, Msg::LegendLater.tr(lang));
        line(DANGER, Msg::LegendAlert.tr(lang));
        line(CLAMP, Msg::LegendClamp.tr(lang));
        line(EDGE, Msg::LegendTable.tr(lang));
    }

    fn dialog_window(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        match show_dialog(ctx, &mut dialog, self.win.lang) {
            None => self.dialog = Some(dialog),
            Some(Outcome::Cancel) => {}
            Some(Outcome::Pick(path)) => {
                if self.win.open_program(&path) {
                    self.fit_pending = true;
                    self.scroll_pending = true;
                } else {
                    // the window stays open so that another file can be tried
                    self.dialog = Some(dialog);
                }
            }
        }
    }
}

/// The word of a dialect.
fn dialect_label(d: Dialect) -> Msg {
    match d {
        Dialect::Grbl => Msg::DialectGrbl,
        Dialect::Marlin => Msg::DialectMarlin,
        Dialect::PlanetCnc => Msg::DialectPlanetCnc,
    }
}

/// Draws the window that opens a file and says what the person chose.
fn show_dialog(ctx: &egui::Context, d: &mut Dialog, lang: Lang) -> Option<Outcome> {
    let mut outcome = None;
    let mut open = true;
    egui::Window::new(Msg::TitleOpen.tr(lang))
        .open(&mut open)
        .collapsible(false)
        .default_size([620.0, 460.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(Msg::Up.tr(lang)).clicked() {
                    d.browser.up();
                }
                let edit = egui::TextEdit::singleline(&mut d.browser.path_text)
                    .desired_width(ui.available_width());
                let response = ui.add(edit);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Some(p) = d.browser.go_typed() {
                        outcome = Some(Outcome::Pick(p));
                    }
                }
            });
            if let Some(e) = &d.browser.error {
                ui.colored_label(DANGER, e.as_str());
            }
            if ui
                .checkbox(&mut d.browser.show_all, Msg::AllFiles.tr(lang))
                .changed()
            {
                d.browser.refresh();
            }
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    if d.browser.entries.is_empty() {
                        ui.label(egui::RichText::new(Msg::EmptyFolder.tr(lang)).color(DIM));
                    }
                    for e in d.browser.entries.clone() {
                        let label = if e.is_dir {
                            format!("{}/", e.name)
                        } else {
                            format!("{}    ({})", e.name, browse::size_text(e.size))
                        };
                        if !ui.selectable_label(false, label).clicked() {
                            continue;
                        }
                        if let Some(p) = d.browser.enter(&e.name) {
                            outcome = Some(Outcome::Pick(p));
                        }
                    }
                });
            ui.separator();
            if ui.button(Msg::Cancel.tr(lang)).clicked() {
                outcome = Some(Outcome::Cancel);
            }
        });
    if !open && outcome.is_none() {
        outcome = Some(Outcome::Cancel);
    }
    outcome
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // a program dropped on the window is opened
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        if let Some(p) = dropped.first() {
            if self.win.open_dropped(p) {
                self.fit_pending = true;
                self.scroll_pending = true;
            }
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
            self.ask();
        }
        // while it plays, the shown move goes forward with the time that passed
        if self.win.playing {
            let dt = ctx.input(|i| i.stable_dt);
            self.win.tick(f64::from(dt));
            ctx.request_repaint();
        }
        self.top_bar(ui);
        self.status_bar(ui);
        self.controls(ui);
        self.side_panel(ui);
        self.steps_panel(ui);
        self.canvas(ui);
        self.dialog_window(&ctx);
        self.shot_tick(&ctx);
        if self.win.current != self.last_current {
            self.last_current = self.win.current;
            self.scroll_pending = true;
        }
    }
}

/// The icon of the program for the window, from the file of the brand.
fn window_icon() -> Option<egui::IconData> {
    let bytes: &[u8] = include_bytes!("../images/brand/icon-256.png");
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buf.truncate(info.buffer_size());
    Some(egui::IconData {
        rgba: buf,
        width: info.width,
        height: info.height,
    })
}

/// The fonts of the system that draw Japanese and Chinese are added to the ones of egui, which have no such letters.
fn install_fonts(ctx: &egui::Context) {
    let candidates = [
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/YuGothM.ttc",
        "C:/Windows/Fonts/meiryo.ttc",
        "C:/Windows/Fonts/simsun.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/System/Library/Fonts/PingFang.ttc",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".into(), Arc::new(FontData::from_owned(bytes)));
            for family in [FontFamily::Monospace, FontFamily::Proportional] {
                fonts.families.entry(family).or_default().push("cjk".into());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

/// The colours of the theme Forge for the controls of egui.
fn apply_theme(ctx: &egui::Context, theme: ShotTheme) {
    match theme {
        ShotTheme::Forge => {}
        ShotTheme::Dark => {
            ctx.set_visuals(egui::Visuals::dark());
            return;
        }
        ShotTheme::Light => {
            ctx.set_visuals(egui::Visuals::light());
            return;
        }
    }
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = RAISED;
    v.extreme_bg_color = Color32::from_rgb(0x14, 0x11, 0x0e);
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0xff, 0xb0, 0x00, 110);
    v.selection.stroke = Stroke::new(1.5, ACCENT);
    v.widgets.inactive.bg_stroke =
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xff, 0xb0, 0x00, 110));
    v.widgets.inactive.weak_bg_fill = RAISED;
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, ACCENT);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x33, 0x28, 0x1c);
    v.widgets.active.bg_stroke = Stroke::new(2.0, ACCENT);
    v.widgets.inactive.corner_radius = 6.0.into();
    v.widgets.hovered.corner_radius = 6.0.into();
    v.widgets.active.corner_radius = 6.0.into();
    ctx.set_visuals(v);
}

/// Opens the window. A program given here is opened at once.
pub fn run(initial: Option<PathBuf>, lang: Lang) -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 780.0])
        .with_min_inner_size([860.0, 520.0])
        .with_title("EH3D-CAM-DEBUGGER");
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "EH3D-CAM-DEBUGGER",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx, ShotTheme::Forge);
            Ok(Box::new(App::new(lang, initial)))
        }),
    )
}

/// How the picture mode ended: nothing yet, the file was written, or the reason it was not.
type ShotOutcome = Arc<Mutex<Option<Result<(), String>>>>;

/// The picture being taken.
struct ShotState {
    request: ShotRequest,
    frame: u32,
    outcome: ShotOutcome,
}

impl App {
    /// Counts the frames, asks for the picture after the frames waited, writes it and closes the window when it is written or when it does not come.
    fn shot_tick(&mut self, ctx: &egui::Context) {
        let Some(shot) = self.shot.as_mut() else {
            return;
        };
        shot.frame += 1;
        ctx.request_repaint();
        let due = 3 + shot.request.wait;
        let finished = match shot.outcome.lock() {
            Ok(slot) => slot.is_some(),
            Err(_) => true,
        };
        if finished {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if shot.frame == due {
            let (output, replace) = (shot.request.output.clone(), shot.request.replace);
            let (outcome, repaint) = (shot.outcome.clone(), ctx.clone());
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                egui::ScreenshotCallback::new(move |image| {
                    let result = save_shot_png(&image, &output, replace);
                    if let Ok(mut slot) = outcome.lock() {
                        *slot = Some(result);
                    }
                    repaint.request_repaint();
                }),
            ));
        } else if shot.frame > due + PATIENCE_FRAMES {
            if let Ok(mut slot) = shot.outcome.lock() {
                *slot = Some(Err(
                    "the picture did not come back from the window".to_owned()
                ));
            }
        }
    }
}

/// Writes the picture the window gave back as a PNG file, a new file unless `replace` is true.
fn save_shot_png(image: &egui::ColorImage, path: &Path, replace: bool) -> Result<(), String> {
    let mut open = std::fs::OpenOptions::new();
    open.write(true);
    if replace {
        open.create(true).truncate(true);
    } else {
        open.create_new(true);
    }
    let file = open
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        image.size[0] as u32,
        image.size[1] as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    let data: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
    writer.write_image_data(&data).map_err(|e| e.to_string())?;
    // the end of the file is written here, so that a failure is reported and not lost in a drop
    writer.finish().map_err(|e| e.to_string())
}

/// Opens the window at the size asked, shows the files, saves the picture as the request says and closes by itself. The window keeps no settings. The answer is the reason when the
/// picture could not be written.
pub fn run_shot(files: Vec<PathBuf>, lang: Lang, request: ShotRequest) -> Result<(), String> {
    if !request.replace && request.output.exists() {
        return Err(format!(
            "{} exists; give --replace to write over it",
            request.output.display()
        ));
    }
    let folder = request
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf);
    if folder.is_some_and(|dir| !dir.is_dir()) {
        return Err(format!(
            "the folder of {} does not exist",
            request.output.display()
        ));
    }
    if let Some(missing) = files.iter().find(|f| !f.is_file()) {
        return Err(format!("{} is not a file", missing.display()));
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([request.size.0 as f32, request.size.1 as f32])
        .with_title("EH3D-CAM-DEBUGGER");
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let outcome: ShotOutcome = Arc::new(Mutex::new(None));
    let outcome_for_app = outcome.clone();
    let theme = request.theme;
    eframe::run_native(
        "EH3D-CAM-DEBUGGER",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx, theme);
            let mut app = App::new(lang, files.into_iter().next());
            app.shot = Some(ShotState {
                request,
                frame: 0,
                outcome: outcome_for_app,
            });
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| e.to_string())?;
    let taken = outcome.lock().map_err(|e| e.to_string())?.take();
    match taken {
        Some(result) => result,
        None => Err("the window closed before the picture was taken".to_owned()),
    }
}
