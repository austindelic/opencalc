use core::{
    eval_env, parse_statement, simplify_with_env, Env, Expr, ScriptRuntime, ScriptScope, Statement,
    UserFn,
};
use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OpenCalc")
            .with_inner_size([1240.0, 760.0])
            .with_min_inner_size([880.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native("OpenCalc", options, Box::new(|_| Ok(Box::new(App::new()))))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    Eval,
    Assignment,
    Function,
    Script,
    Error,
}

struct HistoryEntry {
    input: String,
    output: String,
    detail: String,
    kind: EntryKind,
}

struct PaletteGroup {
    title: &'static str,
    items: &'static [PaletteItem],
}

struct PaletteItem {
    label: &'static str,
    insert: &'static str,
    hint: &'static str,
}

struct Example {
    title: &'static str,
    expression: &'static str,
}

struct App {
    input: String,
    history: Vec<HistoryEntry>,
    env: Env,
    scripts: ScriptRuntime,
    script_scope: ScriptScope,
    script_input: String,
    status: String,
    last_answer: Option<String>,
}

impl App {
    fn new() -> Self {
        Self {
            input: String::from("diff(sin(x)^2 + cos(x)^2, x)"),
            history: Vec::new(),
            env: Env::new(),
            scripts: ScriptRuntime::new(),
            script_scope: ScriptRuntime::new_scope(),
            script_input: String::from(
                "let n = value(\"sqrt(144)\");\ncalc(\"diff(x^3, x)\") + \" = \" + n",
            ),
            status: String::from("Ready"),
            last_answer: None,
        }
    }

    fn evaluate(&mut self) {
        let input = self.input.trim().to_string();
        if input.is_empty() {
            self.status = String::from("Nothing to evaluate");
            return;
        }

        let entry = self.evaluate_statement(&input);
        if entry.kind != EntryKind::Error {
            self.input.clear();
        }
        self.status = match entry.kind {
            EntryKind::Eval => String::from("Evaluated"),
            EntryKind::Assignment => String::from("Variable saved"),
            EntryKind::Function => String::from("Function saved"),
            EntryKind::Script => String::from("Script ran"),
            EntryKind::Error => String::from("Fix expression and try again"),
        };
        self.history.push(entry);
    }

    fn evaluate_statement(&mut self, input: &str) -> HistoryEntry {
        match parse_statement(input) {
            Ok(Statement::Assign(name, expr)) => {
                let simplified = simplify_with_env(expr, &self.env);
                match eval_env(&simplified, &self.env) {
                    Ok(value) => {
                        let result = format_number(value);
                        self.env.set_var(&name, Expr::Float(value));
                        self.env.set_var("ans", Expr::Float(value));
                        self.last_answer = Some(result.clone());
                        HistoryEntry {
                            input: input.to_string(),
                            output: format!("{name} = {result}"),
                            detail: format!("stored numeric value in {name} and ans"),
                            kind: EntryKind::Assignment,
                        }
                    }
                    Err(_) => {
                        let result = simplified.to_string();
                        self.env.set_var(&name, simplified);
                        self.env.set_var("ans", Expr::Var(name.clone()));
                        self.last_answer = Some(result.clone());
                        HistoryEntry {
                            input: input.to_string(),
                            output: format!("{name} = {result}"),
                            detail: format!("stored symbolic expression in {name}"),
                            kind: EntryKind::Assignment,
                        }
                    }
                }
            }
            Ok(Statement::DefFn(name, params, body)) => {
                let signature = format!("{}({})", name, params.join(", "));
                self.env.set_fn(&name, UserFn { params, body });
                HistoryEntry {
                    input: input.to_string(),
                    output: format!("defined {signature}"),
                    detail: String::from("function available in future expressions"),
                    kind: EntryKind::Function,
                }
            }
            Ok(Statement::Eval(expr)) => {
                let simplified = simplify_with_env(expr, &self.env);
                match eval_env(&simplified, &self.env) {
                    Ok(value) => {
                        let result = format_number(value);
                        self.env.set_var("ans", Expr::Float(value));
                        self.last_answer = Some(result.clone());
                        HistoryEntry {
                            input: input.to_string(),
                            output: result,
                            detail: simplified.to_string(),
                            kind: EntryKind::Eval,
                        }
                    }
                    Err(_) => {
                        let result = simplified.to_string();
                        self.env.set_var("ans", simplified);
                        self.last_answer = Some(result.clone());
                        HistoryEntry {
                            input: input.to_string(),
                            output: result,
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

    fn reset_session(&mut self) {
        self.env = Env::new();
        self.script_scope = ScriptRuntime::new_scope();
        self.last_answer = None;
        self.status = String::from("Session reset");
    }

    fn run_script(&mut self) {
        let source = self.script_input.trim().to_string();
        if source.is_empty() {
            self.status = String::from("No script to run");
            return;
        }

        let entry = match self.scripts.run_with_scope(&source, &mut self.script_scope) {
            Ok(output) => {
                let result = if output.is_empty() {
                    String::from("(unit)")
                } else {
                    output
                };
                self.last_answer = Some(result.clone());
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
    }

    fn insert(&mut self, text: &str) {
        self.input.push_str(text);
    }

    fn set_input(&mut self, text: &str) {
        self.input.clear();
        self.input.push_str(text);
    }

    fn ui_shell(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("workspace_panel")
            .resizable(true)
            .default_size(270.0)
            .size_range(230.0..=360.0)
            .show_inside(ui, |ui| self.workspace_panel(ui));

        egui::Panel::right("tools_panel")
            .resizable(true)
            .default_size(300.0)
            .size_range(250.0..=380.0)
            .show_inside(ui, |ui| self.tools_panel(ui));

        egui::Panel::bottom("command_panel")
            .resizable(false)
            .show_inside(ui, |ui| self.command_panel(ui));

        egui::CentralPanel::default().show_inside(ui, |ui| self.history_panel(ui));
    }

    fn workspace_panel(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Workspace", "variables, functions, session");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            metric(ui, "Entries", self.history.len().to_string());
            metric(ui, "Vars", self.env.vars.len().to_string());
            metric(ui, "Fns", self.env.fns.len().to_string());
        });

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            if action_button(ui, "Reset")
                .on_hover_text("Clear variables and functions")
                .clicked()
            {
                self.reset_session();
            }
            if action_button(ui, "Clear log")
                .on_hover_text("Clear calculation history")
                .clicked()
            {
                self.history.clear();
                self.status = String::from("History cleared");
            }
        });

        ui.add_space(18.0);
        ui.heading("Variables");
        ui.add_space(6.0);
        egui::ScrollArea::vertical()
            .id_salt("workspace_vars")
            .max_height(180.0)
            .show(ui, |ui| {
                if self.env.vars.is_empty() {
                    empty_state(ui, "No variables yet");
                }
                for (name, expr) in &self.env.vars {
                    variable_row(ui, name, &expr.to_string(), &mut self.input);
                }
            });

        ui.add_space(18.0);
        ui.heading("Functions");
        ui.add_space(6.0);
        egui::ScrollArea::vertical()
            .id_salt("workspace_fns")
            .max_height(180.0)
            .show(ui, |ui| {
                if self.env.fns.is_empty() {
                    empty_state(ui, "No functions yet");
                }
                for (name, function) in &self.env.fns {
                    let signature = format!("{}({})", name, function.params.join(", "));
                    variable_row(ui, &signature, &function.body.to_string(), &mut self.input);
                }
            });

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.label(egui::RichText::new(&self.status).size(12.0));
        });
    }

    fn tools_panel(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Tools", "examples and function palette");
        ui.add_space(10.0);

        ui.heading("Rhai Script");
        ui.add_space(6.0);
        ui.add(
            egui::TextEdit::multiline(&mut self.script_input)
                .desired_rows(8)
                .desired_width(f32::INFINITY)
                .font(egui::TextStyle::Monospace)
                .hint_text("calc(\"2^10\")\nvalue(\"sqrt(144)\") + 1"),
        );
        ui.horizontal(|ui| {
            if primary_button(ui, "Run Script").clicked() {
                self.run_script();
            }
            if action_button(ui, "Example").clicked() {
                self.script_input = String::from(
                    "let n = value(\"sqrt(144)\");\ncalc(\"diff(x^3, x)\") + \" = \" + n",
                );
            }
            if action_button(ui, "Clear").clicked() {
                self.script_input.clear();
            }
        });
        ui.label(
            egui::RichText::new("Available: calc(text), simplify(text), value(text)").size(11.0),
        );

        ui.add_space(16.0);
        ui.heading("Quick Start");
        ui.add_space(6.0);
        for example in EXAMPLES {
            if example_button(ui, example).clicked() {
                self.set_input(example.expression);
            }
        }

        ui.add_space(16.0);
        ui.heading("Palette");
        ui.add_space(6.0);
        egui::ScrollArea::vertical()
            .id_salt("tool_palette")
            .show(ui, |ui| {
                for group in PALETTE {
                    ui.collapsing(group.title, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for item in group.items {
                                if palette_button(ui, item).clicked() {
                                    self.insert(item.insert);
                                }
                            }
                        });
                    });
                }
            });
    }

    fn history_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("OpenCalc").size(32.0).strong());
                ui.label(egui::RichText::new("symbolic calculator").size(13.0));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(answer) = &self.last_answer {
                    ui.label(
                        egui::RichText::new(format!("ans = {answer}"))
                            .monospace()
                            .size(13.0),
                    );
                }
            });
        });

        ui.add_space(18.0);
        egui::ScrollArea::vertical()
            .id_salt("history")
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if self.history.is_empty() {
                    welcome(ui, &mut self.input);
                    return;
                }

                for index in 0..self.history.len() {
                    let mut rerun = false;
                    let mut use_input = false;
                    let mut use_output = false;
                    let entry = &self.history[index];
                    history_card(ui, entry, |action| match action {
                        HistoryAction::Rerun => rerun = true,
                        HistoryAction::UseInput => use_input = true,
                        HistoryAction::UseOutput => use_output = true,
                    });
                    if rerun {
                        let input = self.history[index].input.clone();
                        self.set_input(&input);
                        self.evaluate();
                    } else if use_input {
                        let input = self.history[index].input.clone();
                        self.set_input(&input);
                    } else if use_output {
                        let output = self.history[index].output.clone();
                        self.set_input(&output);
                    }
                    ui.add_space(10.0);
                }
            });
    }

    fn command_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.input)
                    .desired_width(f32::INFINITY)
                    .hint_text("type an expression, x = 2, or def f(x) = x^2")
                    .font(egui::TextStyle::Monospace),
            );
            let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            if primary_button(ui, "Evaluate").clicked() || enter {
                self.evaluate();
                response.request_focus();
            }
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            for key in QUICK_KEYS {
                if compact_key(ui, key.label).on_hover_text(key.hint).clicked() {
                    self.insert(key.insert);
                }
            }
            ui.separator();
            if compact_key(ui, "ans").clicked() {
                self.insert("ans");
            }
            if compact_key(ui, "clear").clicked() {
                self.input.clear();
            }
        });
        ui.add_space(10.0);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ui_shell(ui);
    }
}

enum HistoryAction {
    Rerun,
    UseInput,
    UseOutput,
}

fn history_card(ui: &mut egui::Ui, entry: &HistoryEntry, mut on_action: impl FnMut(HistoryAction)) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(entry_kind_label(entry.kind)).size(11.0));
                    ui.label(egui::RichText::new(&entry.input).monospace().size(14.0));
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(&entry.output)
                            .monospace()
                            .size(20.0)
                            .strong(),
                    );
                    if !entry.detail.is_empty() && entry.detail != entry.output {
                        ui.label(egui::RichText::new(&entry.detail).monospace().size(12.0));
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if small_button(ui, "run").clicked() {
                        on_action(HistoryAction::Rerun);
                    }
                    if small_button(ui, "use").clicked() {
                        on_action(HistoryAction::UseInput);
                    }
                    if entry.kind != EntryKind::Error && small_button(ui, "result").clicked() {
                        on_action(HistoryAction::UseOutput);
                    }
                });
            });
        });
}

fn entry_kind_label(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Eval => "expression",
        EntryKind::Assignment => "assignment",
        EntryKind::Function => "function",
        EntryKind::Script => "script",
        EntryKind::Error => "error",
    }
}

fn welcome(ui: &mut egui::Ui, input: &mut String) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Start with a calculation")
                    .size(22.0)
                    .strong(),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(
                    "Use symbolic calculus, exact rational arithmetic, variables, and function definitions.",
                ),
            );
            ui.add_space(16.0);
            ui.horizontal_wrapped(|ui| {
                for example in EXAMPLES.iter().take(5) {
                    if example_button(ui, example).clicked() {
                        input.clear();
                        input.push_str(example.expression);
                    }
                }
            });
        });
}

fn section_title(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(egui::RichText::new(title).size(22.0).strong());
    ui.label(egui::RichText::new(subtitle).size(12.0));
}

fn metric(ui: &mut egui::Ui, label: &str, value: String) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(value).monospace().size(18.0).strong());
                ui.label(egui::RichText::new(label).size(11.0));
            });
        });
}

fn variable_row(ui: &mut egui::Ui, name: &str, value: &str, input: &mut String) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(name).monospace().strong());
                    ui.label(egui::RichText::new(value).monospace().size(11.0));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if small_button(ui, "insert").clicked() {
                        input.push_str(name);
                    }
                });
            });
        });
}

fn empty_state(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(12.0).italics());
}

fn example_button(ui: &mut egui::Ui, example: &Example) -> egui::Response {
    ui.add(egui::Button::new(
        egui::RichText::new(format!("{}  {}", example.title, example.expression))
            .monospace()
            .size(12.0),
    ))
}

fn palette_button(ui: &mut egui::Ui, item: &PaletteItem) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(item.label).monospace().size(12.0))
            .min_size(egui::vec2(72.0, 26.0)),
    )
    .on_hover_text(item.hint)
}

fn action_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(egui::Button::new(
        egui::RichText::new(text).size(12.0).strong(),
    ))
}

fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).size(14.0).strong())
            .min_size(egui::vec2(120.0, 34.0)),
    )
}

fn small_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(egui::Button::new(egui::RichText::new(text).size(11.0)))
}

fn compact_key(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).monospace().size(12.0))
            .min_size(egui::vec2(36.0, 24.0)),
    )
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

struct QuickKey {
    label: &'static str,
    insert: &'static str,
    hint: &'static str,
}

const QUICK_KEYS: &[QuickKey] = &[
    QuickKey {
        label: "(",
        insert: "(",
        hint: "open parenthesis",
    },
    QuickKey {
        label: ")",
        insert: ")",
        hint: "close parenthesis",
    },
    QuickKey {
        label: "^",
        insert: "^",
        hint: "power",
    },
    QuickKey {
        label: "sqrt",
        insert: "sqrt(",
        hint: "square root",
    },
    QuickKey {
        label: "pi",
        insert: "pi",
        hint: "constant pi",
    },
    QuickKey {
        label: "x",
        insert: "x",
        hint: "variable x",
    },
    QuickKey {
        label: "diff",
        insert: "diff(",
        hint: "symbolic derivative",
    },
    QuickKey {
        label: "solve",
        insert: "solve(",
        hint: "solve equation",
    },
];

const EXAMPLES: &[Example] = &[
    Example {
        title: "Derivative",
        expression: "diff(exp(x^2) * sin(x), x)",
    },
    Example {
        title: "Simplify",
        expression: "simplify(sin(x)^2 + cos(x)^2)",
    },
    Example {
        title: "Solve",
        expression: "solve(x^2 == 9, x)",
    },
    Example {
        title: "Taylor",
        expression: "taylor(exp(x), x, 0, 5)",
    },
    Example {
        title: "Matrix",
        expression: "det([[1,2],[3,4]])",
    },
    Example {
        title: "Define",
        expression: "def f(x) = x^2 + 2x + 1",
    },
    Example {
        title: "Assign",
        expression: "a = 100 / 7 + 3/14",
    },
];

const PALETTE: &[PaletteGroup] = &[
    PaletteGroup {
        title: "Constants",
        items: &[
            PaletteItem {
                label: "pi",
                insert: "pi",
                hint: "3.14159...",
            },
            PaletteItem {
                label: "e",
                insert: "e",
                hint: "Euler constant",
            },
            PaletteItem {
                label: "i",
                insert: "i",
                hint: "imaginary unit",
            },
            PaletteItem {
                label: "inf",
                insert: "inf",
                hint: "infinity",
            },
        ],
    },
    PaletteGroup {
        title: "Trig",
        items: &[
            PaletteItem {
                label: "sin",
                insert: "sin(",
                hint: "sine",
            },
            PaletteItem {
                label: "cos",
                insert: "cos(",
                hint: "cosine",
            },
            PaletteItem {
                label: "tan",
                insert: "tan(",
                hint: "tangent",
            },
            PaletteItem {
                label: "asin",
                insert: "asin(",
                hint: "inverse sine",
            },
            PaletteItem {
                label: "acos",
                insert: "acos(",
                hint: "inverse cosine",
            },
            PaletteItem {
                label: "atan",
                insert: "atan(",
                hint: "inverse tangent",
            },
        ],
    },
    PaletteGroup {
        title: "Algebra",
        items: &[
            PaletteItem {
                label: "simplify",
                insert: "simplify(",
                hint: "simplify expression",
            },
            PaletteItem {
                label: "expand",
                insert: "expand(",
                hint: "expand expression",
            },
            PaletteItem {
                label: "solve",
                insert: "solve(",
                hint: "solve expression or equation",
            },
            PaletteItem {
                label: "factorial",
                insert: "factorial(",
                hint: "factorial",
            },
        ],
    },
    PaletteGroup {
        title: "Calculus",
        items: &[
            PaletteItem {
                label: "diff",
                insert: "diff(",
                hint: "symbolic derivative",
            },
            PaletteItem {
                label: "integrate",
                insert: "integrate(",
                hint: "symbolic antiderivative",
            },
            PaletteItem {
                label: "integral",
                insert: "integral(",
                hint: "definite integral",
            },
            PaletteItem {
                label: "taylor",
                insert: "taylor(",
                hint: "Taylor series",
            },
            PaletteItem {
                label: "ndiff",
                insert: "ndiff(",
                hint: "numeric derivative",
            },
        ],
    },
    PaletteGroup {
        title: "Log / Roots",
        items: &[
            PaletteItem {
                label: "exp",
                insert: "exp(",
                hint: "exponential",
            },
            PaletteItem {
                label: "ln",
                insert: "ln(",
                hint: "natural log",
            },
            PaletteItem {
                label: "log",
                insert: "log(",
                hint: "log(base, value)",
            },
            PaletteItem {
                label: "sqrt",
                insert: "sqrt(",
                hint: "square root",
            },
            PaletteItem {
                label: "cbrt",
                insert: "cbrt(",
                hint: "cube root",
            },
        ],
    },
    PaletteGroup {
        title: "Number Theory",
        items: &[
            PaletteItem {
                label: "gcd",
                insert: "gcd(",
                hint: "greatest common divisor",
            },
            PaletteItem {
                label: "lcm",
                insert: "lcm(",
                hint: "least common multiple",
            },
            PaletteItem {
                label: "mod",
                insert: "mod(",
                hint: "modulo",
            },
            PaletteItem {
                label: "isprime",
                insert: "isprime(",
                hint: "prime check",
            },
        ],
    },
    PaletteGroup {
        title: "Matrix / Vector",
        items: &[
            PaletteItem {
                label: "det",
                insert: "det(",
                hint: "determinant",
            },
            PaletteItem {
                label: "tr",
                insert: "tr(",
                hint: "matrix trace",
            },
            PaletteItem {
                label: "transpose",
                insert: "transpose(",
                hint: "matrix transpose",
            },
            PaletteItem {
                label: "zeros",
                insert: "zeros(",
                hint: "zero matrix",
            },
            PaletteItem {
                label: "eye",
                insert: "eye(",
                hint: "identity matrix",
            },
            PaletteItem {
                label: "dot",
                insert: "dot(",
                hint: "dot product",
            },
            PaletteItem {
                label: "norm",
                insert: "norm(",
                hint: "vector norm",
            },
        ],
    },
    PaletteGroup {
        title: "Sequences",
        items: &[
            PaletteItem {
                label: "sum",
                insert: "sum(",
                hint: "summation",
            },
            PaletteItem {
                label: "product",
                insert: "product(",
                hint: "product",
            },
            PaletteItem {
                label: "range",
                insert: "range(",
                hint: "range list",
            },
            PaletteItem {
                label: "len",
                insert: "len(",
                hint: "length",
            },
        ],
    },
];
