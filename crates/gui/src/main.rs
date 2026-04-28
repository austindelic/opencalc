use core::{
    eval_env, parse_statement, simplify_with_env, Env, Expr, ScriptRuntime, ScriptScope,
    Statement, UserFn,
};
use eframe::egui;
use egui::{Color32, RichText, Vec2};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OpenCalc")
            .with_inner_size([860.0, 820.0])
            .with_min_inner_size([520.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("OpenCalc", options, Box::new(|_| Ok(Box::new(App::new()))))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PanelTab { Workspace, Script, Palette }

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryKind { Eval, Assignment, Function, Script, Error }

struct HistoryEntry {
    input: String,
    output: String,
    detail: String,
    kind: EntryKind,
}

struct App {
    input: String,
    result: String,
    history: Vec<HistoryEntry>,
    env: Env,
    scripts: ScriptRuntime,
    script_scope: ScriptScope,
    script_input: String,
    status: String,
    last_answer: Option<String>,
    show_history: bool,
    show_panel: bool,
    panel_tab: PanelTab,
    just_evaluated: bool,
}

// Rust-language color palette
const C_BG: Color32    = Color32::from_rgb(18, 15, 14);   // near-black warm brown
const C_DISP: Color32  = Color32::from_rgb(10, 8, 7);     // almost black
const C_NUM: Color32   = Color32::from_rgb(46, 38, 36);   // dark warm grey
const C_OP: Color32    = Color32::from_rgb(206, 65, 43);  // rust #CE412B
const C_FN: Color32    = Color32::from_rgb(72, 50, 46);   // dark rust tint
const C_SPEC: Color32  = Color32::from_rgb(110, 90, 85);  // warm mid-grey
const C_EXPR: Color32  = Color32::from_rgb(158, 126, 116);// warm muted text
const C_RES: Color32   = Color32::from_rgb(245, 240, 238);// warm white
const C_PANEL: Color32 = Color32::from_rgb(22, 18, 17);   // panel background
const C_SEP: Color32   = Color32::from_rgb(55, 42, 38);   // separator line

impl App {
    fn new() -> Self {
        Self {
            input: String::new(),
            result: String::from("0"),
            history: Vec::new(),
            env: Env::new(),
            scripts: ScriptRuntime::new(),
            script_scope: ScriptRuntime::new_scope(),
            script_input: String::from(
                "let n = value(\"sqrt(144)\");\ncalc(\"diff(x^3, x)\") + \" = \" + n",
            ),
            status: String::from("Ready"),
            last_answer: None,
            show_history: false,
            show_panel: true,
            panel_tab: PanelTab::Workspace,
            just_evaluated: false,
        }
    }

    fn evaluate(&mut self) {
        let raw = self.input.trim().to_string();
        if raw.is_empty() {
            self.status = String::from("Nothing to evaluate");
            return;
        }
        let entry = self.evaluate_statement(&raw);
        self.status = match entry.kind {
            EntryKind::Eval       => String::from("Evaluated"),
            EntryKind::Assignment => String::from("Variable saved"),
            EntryKind::Function   => String::from("Function saved"),
            EntryKind::Script     => String::from("Script ran"),
            EntryKind::Error      => String::from("Fix expression and try again"),
        };
        self.result = entry.output.clone();
        self.history.push(entry);
        self.just_evaluated = true;
    }

    fn evaluate_statement(&mut self, input: &str) -> HistoryEntry {
        match parse_statement(input) {
            Ok(Statement::Assign(name, expr)) => {
                let simplified = simplify_with_env(expr, &self.env);
                match eval_env(&simplified, &self.env) {
                    Ok(v) => {
                        let r = format_number(v);
                        self.env.set_var(&name, Expr::Float(v));
                        self.env.set_var("ans", Expr::Float(v));
                        self.last_answer = Some(r.clone());
                        HistoryEntry {
                            input: input.to_string(),
                            output: format!("{name} = {r}"),
                            detail: format!("stored numeric value in {name} and ans"),
                            kind: EntryKind::Assignment,
                        }
                    }
                    Err(_) => {
                        let r = simplified.to_string();
                        self.env.set_var(&name, simplified);
                        self.env.set_var("ans", Expr::Var(name.clone()));
                        self.last_answer = Some(r.clone());
                        HistoryEntry {
                            input: input.to_string(),
                            output: format!("{name} = {r}"),
                            detail: format!("stored symbolic expression in {name}"),
                            kind: EntryKind::Assignment,
                        }
                    }
                }
            }
            Ok(Statement::DefFn(name, params, body)) => {
                let sig = format!("{}({})", name, params.join(", "));
                self.env.set_fn(&name, UserFn { params, body });
                HistoryEntry {
                    input: input.to_string(),
                    output: format!("defined {sig}"),
                    detail: String::from("function available in future expressions"),
                    kind: EntryKind::Function,
                }
            }
            Ok(Statement::Eval(expr)) => {
                let simplified = simplify_with_env(expr, &self.env);
                match eval_env(&simplified, &self.env) {
                    Ok(v) => {
                        let r = format_number(v);
                        self.env.set_var("ans", Expr::Float(v));
                        self.last_answer = Some(r.clone());
                        HistoryEntry {
                            input: input.to_string(),
                            output: r,
                            detail: simplified.to_string(),
                            kind: EntryKind::Eval,
                        }
                    }
                    Err(_) => {
                        let r = simplified.to_string();
                        self.env.set_var("ans", simplified);
                        self.last_answer = Some(r.clone());
                        HistoryEntry {
                            input: input.to_string(),
                            output: r,
                            detail: String::from("symbolic result"),
                            kind: EntryKind::Eval,
                        }
                    }
                }
            }
            Err(err) => HistoryEntry {
                input: input.to_string(),
                output: format!("error: {err}"),
                detail: String::from("parser rejected this input"),
                kind: EntryKind::Error,
            },
        }
    }

    fn run_script(&mut self) {
        let source = self.script_input.trim().to_string();
        if source.is_empty() {
            self.status = String::from("No script to run");
            return;
        }
        let entry = match self.scripts.run_with_scope(&source, &mut self.script_scope) {
            Ok(output) => {
                let result = if output.is_empty() { String::from("(unit)") } else { output };
                self.last_answer = Some(result.clone());
                self.result = result.clone();
                HistoryEntry {
                    input: source,
                    output: result,
                    detail: String::from("rhai script"),
                    kind: EntryKind::Script,
                }
            }
            Err(err) => HistoryEntry {
                input: source,
                output: format!("error: {err}"),
                detail: String::from("rhai script failed"),
                kind: EntryKind::Error,
            },
        };
        self.status = match entry.kind {
            EntryKind::Error => String::from("Script failed"),
            _ => String::from("Script ran"),
        };
        self.history.push(entry);
        self.just_evaluated = true;
    }

    fn dispatch(&mut self, action: &str) {
        match action {
            "eval" => self.evaluate(),
            "clear" => {
                self.input.clear();
                self.result = String::from("0");
                self.just_evaluated = false;
            }
            "reset" => {
                self.env = Env::new();
                self.script_scope = ScriptRuntime::new_scope();
                self.input.clear();
                self.result = String::from("0");
                self.last_answer = None;
                self.just_evaluated = false;
                self.status = String::from("Session reset");
            }
            "backspace" => {
                if self.just_evaluated {
                    self.input.clear();
                    self.result = String::from("0");
                    self.just_evaluated = false;
                } else {
                    self.input.pop();
                }
            }
            "negate" => {
                if self.input.starts_with('-') {
                    self.input.remove(0);
                } else if !self.input.is_empty() {
                    self.input.insert(0, '-');
                }
            }
            s => {
                let is_op = matches!(s, "+" | "-" | "*" | "/" | "^");
                if self.just_evaluated && !is_op {
                    self.input.clear();
                    self.result = String::from("0");
                }
                self.just_evaluated = false;
                self.input.push_str(s);
            }
        }
    }

    fn insert(&mut self, text: &str) {
        self.just_evaluated = false;
        self.input.push_str(text);
    }

    fn set_input(&mut self, text: &str) {
        self.input.clear();
        self.input.push_str(text);
        self.just_evaluated = false;
    }

    // ---------------------------------------------------------------
    // Top-level layout
    // ---------------------------------------------------------------

    fn ui_shell(&mut self, ui: &mut egui::Ui) {
        ui.ctx().set_visuals(egui::Visuals::dark());
        ui.painter().rect_filled(ui.max_rect(), 0.0, C_BG);

        if self.show_panel {
            egui::Panel::left("side_panel")
                .resizable(true)
                .default_size(290.0)
                .size_range(240.0..=420.0)
                .show_inside(ui, |ui| {
                    ui.painter().rect_filled(ui.max_rect(), 0.0, C_PANEL);
                    self.side_panel(ui);
                });
        }

        egui::Panel::bottom("status_bar")
            .resizable(false)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                left: 10, right: 10, top: 6, bottom: 6,
            }))
            .show_inside(ui, |ui| {
                ui.label(RichText::new(&self.status).size(11.0).color(C_EXPR));
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                left: 0, right: 10, top: 0, bottom: 0,
            }))
            .show_inside(ui, |ui| self.calc_area(ui));
    }

    // ---------------------------------------------------------------
    // Calculator area (right side)
    // ---------------------------------------------------------------

    fn calc_area(&mut self, ui: &mut egui::Ui) {
        self.mode_bar(ui);

        // dock keypad at bottom — never pushed off
        egui::Panel::bottom("keypad_dock")
            .resizable(false)
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                ui.add_space(6.0);
                self.unified_keypad(ui);
            });

        // dock display just above keypad
        egui::Panel::bottom("display_dock")
            .resizable(false)
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                self.display_panel(ui);
                ui.add_space(6.0);
            });

        // history fills whatever vertical space remain, scroll inside
        if self.show_history {
            self.history_strip(ui);
        }
    }

    fn mode_bar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(RichText::new("OpenCalc").size(15.0).color(C_EXPR));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(12.0);

                let pf = if self.show_panel { C_OP } else { C_FN };
                if ui.add(
                    egui::Button::new(RichText::new("Tools").size(11.0).color(C_RES))
                        .fill(pf).min_size(Vec2::new(44.0, 26.0)),
                ).clicked() {
                    self.show_panel = !self.show_panel;
                }

                ui.add_space(4.0);

                let hf = if self.show_history { C_OP } else { C_FN };
                if ui.add(
                    egui::Button::new(RichText::new("Hist").size(11.0).color(C_RES))
                        .fill(hf).min_size(Vec2::new(38.0, 26.0)),
                ).clicked() {
                    self.show_history = !self.show_history;
                }
            });
        });
        ui.add_space(6.0);
    }

    fn display_panel(&mut self, ui: &mut egui::Ui) {
        let mut do_eval = false;
        egui::Frame::default()
            .fill(C_DISP)
            .inner_margin(egui::Margin::symmetric(16_i8, 14_i8))
            .show(ui, |ui| {
                let inner_w = ui.available_width();
                ui.allocate_space(Vec2::new(inner_w, 0.0));
                ui.set_min_height(108.0);

                let expr_size: f32 = if self.input.len() <= 28 { 17.0 }
                    else if self.input.len() <= 45 { 13.0 }
                    else { 10.5 };

                let v = &mut ui.style_mut().visuals;
                v.extreme_bg_color = Color32::TRANSPARENT;
                v.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                v.widgets.hovered.bg_stroke  = egui::Stroke::NONE;
                v.widgets.active.bg_stroke   = egui::Stroke::NONE;
                v.selection.stroke           = egui::Stroke::NONE;

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.input)
                            .desired_width(inner_w)
                            .horizontal_align(egui::Align::Max)
                            .text_color(C_EXPR)
                            .font(egui::FontId::monospace(expr_size))
                            .hint_text("type expression…"),
                    );
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        do_eval = true;
                    }
                });

                ui.add_space(6.0);

                let res_size: f32 = if self.result.len() <= 10 { 44.0 }
                    else if self.result.len() <= 18 { 32.0 }
                    else if self.result.len() <= 28 { 22.0 }
                    else { 15.0 };
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    ui.label(RichText::new(&self.result).color(C_RES).size(res_size).strong());
                });
            });
        if do_eval { self.evaluate(); }
    }

    fn history_strip(&mut self, ui: &mut egui::Ui) {
        let avail_w = ui.available_width();
        egui::Frame::default()
            .fill(Color32::from_rgb(14, 11, 10))
            .show(ui, |ui| {
                ui.allocate_space(Vec2::new(avail_w, 0.0));
                egui::ScrollArea::vertical()
                    .id_salt("hist_strip")
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.add_space(8.0);
                        if self.history.is_empty() {
                            welcome(ui, &mut self.input);
                            return;
                        }
                        for index in 0..self.history.len() {
                            let mut rerun = false;
                            let mut use_input = false;
                            let mut use_output = false;
                            history_card(ui, &self.history[index], |action| match action {
                                HistoryAction::Rerun     => rerun = true,
                                HistoryAction::UseInput  => use_input = true,
                                HistoryAction::UseOutput => use_output = true,
                            });
                            ui.add_space(5.0);
                            if rerun {
                                let inp = self.history[index].input.clone();
                                self.set_input(&inp);
                                self.evaluate();
                            } else if use_input {
                                let inp = self.history[index].input.clone();
                                self.set_input(&inp);
                            } else if use_output {
                                let out = self.history[index].output.clone();
                                self.set_input(&out);
                            }
                        }
                        ui.add_space(8.0);
                    });
            });
    }

    // ---------------------------------------------------------------
    // Side panel (left side)
    // ---------------------------------------------------------------

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            for (label, tab) in [
                ("Workspace", PanelTab::Workspace),
                ("Script", PanelTab::Script),
                ("Palette", PanelTab::Palette),
            ] {
                let selected = self.panel_tab == tab;
                let fill = if selected { C_OP } else { C_FN };
                let tc = if selected { C_RES } else { C_EXPR };
                if ui.add(
                    egui::Button::new(RichText::new(label).size(11.0).color(tc).strong())
                        .fill(fill).min_size(Vec2::new(0.0, 24.0)),
                ).clicked() {
                    self.panel_tab = tab;
                }
            }
        });
        ui.add_space(8.0);
        ui.painter().hline(
            ui.max_rect().x_range(),
            ui.cursor().min.y,
            egui::Stroke::new(1.0, C_SEP),
        );
        ui.add_space(8.0);

        match self.panel_tab {
            PanelTab::Workspace => self.workspace_tab(ui),
            PanelTab::Script    => self.script_tab(ui),
            PanelTab::Palette   => self.palette_tab(ui),
        }
    }

    fn workspace_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("Workspace").size(18.0).strong());
                ui.label(RichText::new("variables, functions, session").size(11.0).color(C_EXPR));
            });
        });
        ui.add_space(10.0);

        // Stats row — inline text, not large frames
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            stat_badge(ui, "Entries", self.history.len());
            ui.add_space(6.0);
            stat_badge(ui, "Vars", self.env.vars.len());
            ui.add_space(6.0);
            stat_badge(ui, "Fns", self.env.fns.len());
        });

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            if action_button(ui, "Reset").on_hover_text("Clear variables and functions").clicked() {
                self.dispatch("reset");
            }
            ui.add_space(4.0);
            if action_button(ui, "Clear log").on_hover_text("Clear calculation history").clicked() {
                self.history.clear();
                self.status = String::from("History cleared");
            }
        });

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.heading("Variables");
        });
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .id_salt("ws_vars")
            .max_height(150.0)
            .show(ui, |ui| {
                ui.add_space(2.0);
                if self.env.vars.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        empty_state(ui, "No variables yet");
                    });
                }
                for (name, expr) in &self.env.vars {
                    variable_row(ui, name, &expr.to_string(), &mut self.input);
                }
            });

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.heading("Functions");
        });
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .id_salt("ws_fns")
            .max_height(150.0)
            .show(ui, |ui| {
                ui.add_space(2.0);
                if self.env.fns.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        empty_state(ui, "No functions yet");
                    });
                }
                for (name, func) in &self.env.fns {
                    let sig = format!("{}({})", name, func.params.join(", "));
                    variable_row(ui, &sig, &func.body.to_string(), &mut self.input);
                }
            });

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(RichText::new(&self.status).size(11.0).color(C_EXPR));
            });
        });
    }

    fn script_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("Rhai Script").size(18.0).strong());
                ui.label(
                    RichText::new("run programmatic expressions").size(11.0).color(C_EXPR),
                );
            });
        });
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.add(
                egui::TextEdit::multiline(&mut self.script_input)
                    .desired_rows(8)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("calc(\"2^10\")\nvalue(\"sqrt(144)\") + 1"),
            );
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            if primary_button(ui, "Run Script").clicked() { self.run_script(); }
            ui.add_space(4.0);
            if action_button(ui, "Example").clicked() {
                self.script_input = String::from(
                    "let n = value(\"sqrt(144)\");\ncalc(\"diff(x^3, x)\") + \" = \" + n",
                );
            }
            ui.add_space(4.0);
            if action_button(ui, "Clear").clicked() { self.script_input.clear(); }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(
                RichText::new("Available: calc(text), simplify(text), value(text)")
                    .size(10.0).color(C_EXPR),
            );
        });

        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.heading("Quick Start");
        });
        ui.add_space(6.0);
        for example in EXAMPLES {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                if example_button(ui, example).clicked() {
                    self.set_input(example.expression);
                }
            });
        }
    }

    fn palette_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("Palette").size(18.0).strong());
                ui.label(RichText::new("click to insert").size(11.0).color(C_EXPR));
            });
        });
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .id_salt("palette_scroll")
            .show(ui, |ui| {
                for group in PALETTE {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        ui.collapsing(group.title, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                for item in group.items {
                                    if palette_button(ui, item).clicked() {
                                        self.insert(item.insert);
                                    }
                                }
                            });
                        });
                    });
                }
            });
    }

    // ---------------------------------------------------------------
    // Calculator buttons
    // ---------------------------------------------------------------

    fn unified_keypad(&mut self, ui: &mut egui::Ui) {
        const FN_ROWS: &[[(&str, &str); 5]] = &[
            [("diff(",  "diff("),  ("∫",      "integrate("), ("solve(", "solve("), ("taylor(","taylor("), ("n!", "!")],
            [("sin(",   "sin("),   ("cos(",   "cos("),       ("tan(",   "tan("),   ("ln(",    "ln("),     ("√",  "sqrt(")],
            [("asin(",  "asin("),  ("acos(",  "acos("),      ("atan(",  "atan("),  ("log(",   "log("),    ("^",  "^")],
            [("π",      "pi"),     ("e",      "e"),          ("x",      "x"),      ("ans",    "ans"),     ("(",  "(")],
        ];
        const NUM_ROWS: &[[(&str, u8, &str); 4]] = &[
            [("AC", 2, "clear"), ("+/-", 2, "negate"), ("%", 2, "%"), ("÷", 1, "/")],
            [("7",  0, "7"),     ("8",   0, "8"),      ("9", 0, "9"), ("×", 1, "*")],
            [("4",  0, "4"),     ("5",   0, "5"),      ("6", 0, "6"), ("−", 1, "-")],
            [("1",  0, "1"),     ("2",   0, "2"),      ("3", 0, "3"), ("+", 1, "+")],
        ];
        let num_color = |c: u8| match c { 1 => C_OP, 2 => C_SPEC, _ => C_NUM };

        // kill default item_spacing — we control every gap explicitly with add_space
        ui.spacing_mut().item_spacing = Vec2::ZERO;

        let pad = 10.0;
        let gap = 5.0;
        let h = 56.0;
        // 9 button columns total + 8 inter-button gaps + 2 outer pad + 1 column-separator gap
        let total = ui.available_width() - 2.0 * pad - 8.0 * gap - gap;
        let fn_w  = (total * 5.0 / 9.0 / 5.0).floor();
        let num_w = (total * 4.0 / 9.0 / 4.0).floor();

        let mut action: Option<&str> = None;

        ui.horizontal(|ui| {
            ui.add_space(pad);

            // ── left: function grid 5×4 ──
            ui.vertical(|ui| {
                for row in FN_ROWS {
                    ui.horizontal(|ui| {
                        for (i, (label, ins)) in row.iter().enumerate() {
                            if calc_btn(ui, label, C_FN, Vec2::new(fn_w, h)).clicked() {
                                action = Some(*ins);
                            }
                            if i < 4 { ui.add_space(gap); }
                        }
                    });
                    ui.add_space(gap);
                }
            });

            ui.add_space(gap);

            // ── right: number pad 4×4 + 0/./= row ──
            ui.vertical(|ui| {
                for row in NUM_ROWS {
                    ui.horizontal(|ui| {
                        for (i, (label, c, act)) in row.iter().enumerate() {
                            if calc_btn(ui, label, num_color(*c), Vec2::new(num_w, h)).clicked() {
                                action = Some(*act);
                            }
                            if i < 3 { ui.add_space(gap); }
                        }
                    });
                    ui.add_space(gap);
                }
                ui.horizontal(|ui| {
                    if calc_btn(ui, "0", C_NUM, Vec2::new(num_w * 2.0 + gap, h)).clicked() { action = Some("0"); }
                    ui.add_space(gap);
                    if calc_btn(ui, ".", C_NUM, Vec2::new(num_w, h)).clicked() { action = Some("."); }
                    ui.add_space(gap);
                    if calc_btn(ui, "=", C_OP,  Vec2::new(num_w, h)).clicked() { action = Some("eval"); }
                });
            });

            ui.add_space(pad);
        });
        ui.add_space(pad);

        if let Some(a) = action { self.dispatch(a); }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ui_shell(ui);
    }
}

// ---------------------------------------------------------------
// History widgets
// ---------------------------------------------------------------

enum HistoryAction { Rerun, UseInput, UseOutput }

fn history_card(
    ui: &mut egui::Ui,
    entry: &HistoryEntry,
    mut on_action: impl FnMut(HistoryAction),
) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(9_i8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    let kind_color = match entry.kind {
                        EntryKind::Error    => Color32::from_rgb(220, 80, 60),
                        EntryKind::Function => Color32::from_rgb(120, 170, 255),
                        EntryKind::Script   => Color32::from_rgb(180, 140, 255),
                        _                   => C_EXPR,
                    };
                    ui.label(RichText::new(entry_kind_label(entry.kind)).size(10.0).color(kind_color));
                    ui.label(RichText::new(&entry.input).monospace().size(12.0));
                    ui.add_space(2.0);
                    let out_size: f32 = if entry.output.len() <= 20 { 16.0 } else { 12.0 };
                    ui.label(RichText::new(&entry.output).monospace().size(out_size).strong());
                    if !entry.detail.is_empty() && entry.detail != entry.output {
                        ui.label(RichText::new(&entry.detail).monospace().size(10.0).color(C_EXPR));
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if small_button(ui, "run").clicked()    { on_action(HistoryAction::Rerun); }
                    if small_button(ui, "use").clicked()    { on_action(HistoryAction::UseInput); }
                    if entry.kind != EntryKind::Error {
                        if small_button(ui, "result").clicked() { on_action(HistoryAction::UseOutput); }
                    }
                });
            });
        });
}

fn entry_kind_label(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Eval       => "expression",
        EntryKind::Assignment => "assignment",
        EntryKind::Function   => "function",
        EntryKind::Script     => "script",
        EntryKind::Error      => "error",
    }
}

fn welcome(ui: &mut egui::Ui, input: &mut String) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(12_i8))
        .show(ui, |ui| {
            ui.label(RichText::new("Start with a calculation").size(16.0).strong());
            ui.add_space(4.0);
            ui.label(
                RichText::new("Symbolic calculus, exact rationals, variables, functions.")
                    .size(11.0).color(C_EXPR),
            );
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                for ex in EXAMPLES.iter().take(5) {
                    if example_button(ui, ex).clicked() {
                        input.clear();
                        input.push_str(ex.expression);
                    }
                }
            });
        });
}

// ---------------------------------------------------------------
// Button / widget helpers
// ---------------------------------------------------------------

fn calc_btn(ui: &mut egui::Ui, label: &str, fill: Color32, size: Vec2) -> egui::Response {
    let tc = if fill == C_SPEC { Color32::from_rgb(20, 14, 12) } else { C_RES };
    ui.add(
        egui::Button::new(RichText::new(label).size(17.0).color(tc).strong())
            .fill(fill)
            .min_size(size),
    )
}

fn stat_badge(ui: &mut egui::Ui, label: &str, value: usize) {
    egui::Frame::default()
        .fill(Color32::from_rgb(35, 28, 26))
        .inner_margin(egui::Margin::symmetric(8_i8, 4_i8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{value}")).monospace().size(14.0).strong());
                ui.add_space(3.0);
                ui.label(RichText::new(label).size(11.0).color(C_EXPR));
            });
        });
}

fn variable_row(ui: &mut egui::Ui, name: &str, value: &str, input: &mut String) {
    egui::Frame::default()
        .fill(Color32::from_rgb(28, 22, 20))
        .inner_margin(egui::Margin::symmetric(8_i8, 5_i8))
        .show(ui, |ui| {
            ui.allocate_space(Vec2::new(ui.available_width(), 0.0));
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(name).monospace().strong().size(12.0));
                    ui.label(RichText::new(value).monospace().size(10.0).color(C_EXPR));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if small_button(ui, "insert").clicked() { input.push_str(name); }
                });
            });
        });
}

fn empty_state(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).italics().color(C_EXPR));
}

fn example_button(ui: &mut egui::Ui, example: &Example) -> egui::Response {
    ui.add(egui::Button::new(
        RichText::new(format!("{}  {}", example.title, example.expression))
            .monospace().size(11.0),
    ))
}

fn palette_button(ui: &mut egui::Ui, item: &PaletteItem) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(item.label).monospace().size(11.0))
            .min_size(Vec2::new(68.0, 24.0)),
    )
    .on_hover_text(item.hint)
}

fn action_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(egui::Button::new(
        RichText::new(text).size(12.0).strong().color(C_RES),
    ).fill(C_FN))
}

fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(13.0).strong().color(C_RES))
            .fill(C_OP)
            .min_size(Vec2::new(120.0, 30.0)),
    )
}

fn small_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(text).size(10.0).color(C_EXPR)))
}

fn format_number(value: f64) -> String {
    if value.fract().abs() < 1e-12 && value.abs() < 9_007_199_254_740_992.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.12}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

// ---------------------------------------------------------------
// Static data
// ---------------------------------------------------------------

struct Example    { title: &'static str, expression: &'static str }
struct PaletteGroup { title: &'static str, items: &'static [PaletteItem] }
struct PaletteItem  { label: &'static str, insert: &'static str, hint: &'static str }

const EXAMPLES: &[Example] = &[
    Example { title: "Derivative", expression: "diff(exp(x^2) * sin(x), x)" },
    Example { title: "Simplify",   expression: "simplify(sin(x)^2 + cos(x)^2)" },
    Example { title: "Solve",      expression: "solve(x^2 == 9, x)" },
    Example { title: "Taylor",     expression: "taylor(exp(x), x, 0, 5)" },
    Example { title: "Matrix",     expression: "det([[1,2],[3,4]])" },
    Example { title: "Define",     expression: "def f(x) = x^2 + 2x + 1" },
    Example { title: "Assign",     expression: "a = 100 / 7 + 3/14" },
];

const PALETTE: &[PaletteGroup] = &[
    PaletteGroup {
        title: "Constants",
        items: &[
            PaletteItem { label: "pi",  insert: "pi",  hint: "3.14159…" },
            PaletteItem { label: "e",   insert: "e",   hint: "Euler constant" },
            PaletteItem { label: "i",   insert: "i",   hint: "imaginary unit" },
            PaletteItem { label: "inf", insert: "inf", hint: "infinity" },
        ],
    },
    PaletteGroup {
        title: "Trig",
        items: &[
            PaletteItem { label: "sin",  insert: "sin(",  hint: "sine" },
            PaletteItem { label: "cos",  insert: "cos(",  hint: "cosine" },
            PaletteItem { label: "tan",  insert: "tan(",  hint: "tangent" },
            PaletteItem { label: "asin", insert: "asin(", hint: "inverse sine" },
            PaletteItem { label: "acos", insert: "acos(", hint: "inverse cosine" },
            PaletteItem { label: "atan", insert: "atan(", hint: "inverse tangent" },
        ],
    },
    PaletteGroup {
        title: "Algebra",
        items: &[
            PaletteItem { label: "simplify",  insert: "simplify(",  hint: "simplify expression" },
            PaletteItem { label: "expand",    insert: "expand(",    hint: "expand expression" },
            PaletteItem { label: "solve",     insert: "solve(",     hint: "solve equation" },
            PaletteItem { label: "factorial", insert: "factorial(", hint: "factorial" },
        ],
    },
    PaletteGroup {
        title: "Calculus",
        items: &[
            PaletteItem { label: "diff",      insert: "diff(",      hint: "symbolic derivative" },
            PaletteItem { label: "integrate", insert: "integrate(", hint: "symbolic antiderivative" },
            PaletteItem { label: "integral",  insert: "integral(",  hint: "definite integral" },
            PaletteItem { label: "taylor",    insert: "taylor(",    hint: "Taylor series" },
            PaletteItem { label: "ndiff",     insert: "ndiff(",     hint: "numeric derivative" },
        ],
    },
    PaletteGroup {
        title: "Log / Roots",
        items: &[
            PaletteItem { label: "exp",  insert: "exp(",  hint: "exponential e^x" },
            PaletteItem { label: "ln",   insert: "ln(",   hint: "natural logarithm" },
            PaletteItem { label: "log",  insert: "log(",  hint: "log(base, value)" },
            PaletteItem { label: "sqrt", insert: "sqrt(", hint: "square root" },
            PaletteItem { label: "cbrt", insert: "cbrt(", hint: "cube root" },
        ],
    },
    PaletteGroup {
        title: "Number Theory",
        items: &[
            PaletteItem { label: "gcd",     insert: "gcd(",     hint: "greatest common divisor" },
            PaletteItem { label: "lcm",     insert: "lcm(",     hint: "least common multiple" },
            PaletteItem { label: "mod",     insert: "mod(",     hint: "modulo" },
            PaletteItem { label: "isprime", insert: "isprime(", hint: "primality check" },
        ],
    },
    PaletteGroup {
        title: "Matrix / Vector",
        items: &[
            PaletteItem { label: "det",       insert: "det(",       hint: "determinant" },
            PaletteItem { label: "tr",        insert: "tr(",        hint: "matrix trace" },
            PaletteItem { label: "transpose", insert: "transpose(", hint: "matrix transpose" },
            PaletteItem { label: "zeros",     insert: "zeros(",     hint: "zero matrix" },
            PaletteItem { label: "eye",       insert: "eye(",       hint: "identity matrix" },
            PaletteItem { label: "dot",       insert: "dot(",       hint: "dot product" },
            PaletteItem { label: "norm",      insert: "norm(",      hint: "vector norm" },
        ],
    },
    PaletteGroup {
        title: "Sequences",
        items: &[
            PaletteItem { label: "sum",     insert: "sum(",     hint: "summation" },
            PaletteItem { label: "product", insert: "product(", hint: "product" },
            PaletteItem { label: "range",   insert: "range(",   hint: "range list" },
            PaletteItem { label: "len",     insert: "len(",     hint: "length" },
        ],
    },
];
