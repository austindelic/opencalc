use eframe::egui;
use core::{
    parse_statement, simplify, eval_env, Statement, Env, UserFn, Expr,
};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("opencalc")
            .with_inner_size([1100.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native("opencalc", options, Box::new(|_| Ok(Box::new(App::new()))))
}

// ── App state ─────────────────────────────────────────────────────────────────

struct App {
    input:   String,
    history: Vec<(String, String)>,
    env:     Env,
}

impl App {
    fn new() -> Self {
        App {
            input:   String::new(),
            history: Vec::new(),
            env:     Env::new(),
        }
    }

    fn evaluate(&mut self) {
        let input = self.input.trim().to_string();
        if input.is_empty() { return; }

        let display = match parse_statement(&input) {
            Ok(Statement::Assign(name, expr)) => {
                let simplified = simplify(expr);
                match eval_env(&simplified, &self.env) {
                    Ok(v) => {
                        self.env.set_var(&name, Expr::Float(v));
                        format!("{} = {}", name, v)
                    }
                    Err(_) => {
                        self.env.set_var(&name, simplified.clone());
                        format!("{} = {}", name, simplified)
                    }
                }
            }
            Ok(Statement::DefFn(name, params, body)) => {
                let n = params.len();
                self.env.set_fn(&name, UserFn { params, body });
                format!("defined {}({} param{})", name, n, if n == 1 { "" } else { "s" })
            }
            Ok(Statement::Eval(expr)) => {
                let simplified = simplify(expr);
                match eval_env(&simplified, &self.env) {
                    Ok(v)  => format!("{}", v),
                    Err(_) => format!("{}", simplified),
                }
            }
            Err(e) => format!("error: {}", e),
        };

        self.history.push((input, display));
        self.input.clear();
    }
}

// ── Function palette ──────────────────────────────────────────────────────────

fn fn_button(ui: &mut egui::Ui, label: &str, insert: &str, input: &mut String) {
    if ui.add(egui::Button::new(
        egui::RichText::new(label).monospace().size(11.5)
    ).min_size(egui::vec2(74.0, 20.0))).clicked() {
        input.push_str(insert);
    }
}

fn const_button(ui: &mut egui::Ui, label: &str, insert: &str, input: &mut String) {
    if ui.add(egui::Button::new(
        egui::RichText::new(label).monospace().size(11.5)
    ).min_size(egui::vec2(74.0, 20.0))).clicked() {
        input.push_str(insert);
    }
}

fn function_palette(ui: &mut egui::Ui, input: &mut String) {
    egui::ScrollArea::vertical().id_salt("fn_palette").show(ui, |ui| {
        ui.collapsing("Constants", |ui| {
            ui.horizontal_wrapped(|ui| {
                const_button(ui, "π",   "pi",       input);
                const_button(ui, "e",   "e",        input);
                const_button(ui, "i",   "i",        input);
                const_button(ui, "∞",   "inf",      input);
            });
        });

        ui.collapsing("Trig", |ui| {
            ui.horizontal_wrapped(|ui| {
                fn_button(ui, "sin",   "sin(",   input);
                fn_button(ui, "cos",   "cos(",   input);
                fn_button(ui, "tan",   "tan(",   input);
                fn_button(ui, "asin",  "asin(",  input);
                fn_button(ui, "acos",  "acos(",  input);
                fn_button(ui, "atan",  "atan(",  input);
                fn_button(ui, "atan2", "atan2(", input);
            });
        });

        ui.collapsing("Hyperbolic", |ui| {
            ui.horizontal_wrapped(|ui| {
                fn_button(ui, "sinh",  "sinh(",  input);
                fn_button(ui, "cosh",  "cosh(",  input);
                fn_button(ui, "tanh",  "tanh(",  input);
                fn_button(ui, "asinh", "asinh(", input);
                fn_button(ui, "acosh", "acosh(", input);
                fn_button(ui, "atanh", "atanh(", input);
            });
        });

        ui.collapsing("Exp / Log", |ui| {
            ui.horizontal_wrapped(|ui| {
                fn_button(ui, "exp",   "exp(",   input);
                fn_button(ui, "ln",    "ln(",    input);
                fn_button(ui, "log",   "log(",   input);
                fn_button(ui, "log2",  "log2(",  input);
                fn_button(ui, "log10", "log10(", input);
            });
        });

        ui.collapsing("Roots / Abs", |ui| {
            ui.horizontal_wrapped(|ui| {
                fn_button(ui, "sqrt", "sqrt(", input);
                fn_button(ui, "cbrt", "cbrt(", input);
                fn_button(ui, "abs",  "abs(",  input);
            });
        });

        ui.collapsing("Rounding", |ui| {
            ui.horizontal_wrapped(|ui| {
                fn_button(ui, "floor", "floor(", input);
                fn_button(ui, "ceil",  "ceil(",  input);
                fn_button(ui, "round", "round(", input);
                fn_button(ui, "sign",  "sign(",  input);
            });
        });

        ui.collapsing("Number Theory", |ui| {
            ui.horizontal_wrapped(|ui| {
                fn_button(ui, "gcd",     "gcd(",     input);
                fn_button(ui, "lcm",     "lcm(",     input);
                fn_button(ui, "mod",     "mod(",     input);
                fn_button(ui, "isprime", "isprime(", input);
                fn_button(ui, "max",     "max(",     input);
                fn_button(ui, "min",     "min(",     input);
                fn_button(ui, "numer",   "numer(",   input);
                fn_button(ui, "denom",   "denom(",   input);
            });
        });

        ui.collapsing("Calculus", |ui| {
            ui.horizontal_wrapped(|ui| {
                fn_button(ui, "diff",       "diff(",       input);
                fn_button(ui, "integrate",  "integrate(",  input);
                fn_button(ui, "ndiff",      "ndiff(",      input);
                fn_button(ui, "nintegrate", "nintegrate(", input);
                fn_button(ui, "solve",      "solve(",      input);
                fn_button(ui, "taylor",     "taylor(",     input);
                fn_button(ui, "expand",     "expand(",     input);
                fn_button(ui, "simplify",   "simplify(",   input);
            });
        });

        ui.collapsing("Matrix", |ui| {
            ui.horizontal_wrapped(|ui| {
                fn_button(ui, "det",       "det(",       input);
                fn_button(ui, "tr",        "tr(",        input);
                fn_button(ui, "transpose", "transpose(", input);
                fn_button(ui, "inv",       "inv(",       input);
                fn_button(ui, "zeros",     "zeros(",     input);
                fn_button(ui, "ones",      "ones(",      input);
                fn_button(ui, "eye",       "eye(",       input);
                fn_button(ui, "dot",       "dot(",       input);
                fn_button(ui, "cross",     "cross(",     input);
                fn_button(ui, "norm",      "norm(",      input);
            });
        });

        ui.collapsing("Sequences", |ui| {
            ui.horizontal_wrapped(|ui| {
                fn_button(ui, "sum",     "sum(",     input);
                fn_button(ui, "product", "product(", input);
                fn_button(ui, "range",   "range(",   input);
                fn_button(ui, "len",     "len(",     input);
            });
        });

        ui.collapsing("Complex", |ui| {
            ui.horizontal_wrapped(|ui| {
                fn_button(ui, "re",   "re(",   input);
                fn_button(ui, "im",   "im(",   input);
                fn_button(ui, "conj", "conj(", input);
                fn_button(ui, "arg",  "arg(",  input);
            });
        });

        ui.collapsing("Misc", |ui| {
            ui.horizontal_wrapped(|ui| {
                fn_button(ui, "random",    "random(",    input);
                fn_button(ui, "if",        "if(",        input);
                fn_button(ui, "factorial", "factorial(", input);
            });
        });
    });
}

// ── eframe UI ─────────────────────────────────────────────────────────────────

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::left("sidebar")
            .min_size(200.0)
            .max_size(240.0)
            .show_inside(ui, |ui| {
                ui.heading("Variables");
                ui.separator();
                egui::ScrollArea::vertical().id_salt("vars").show(ui, |ui| {
                    for (name, val) in &self.env.vars {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("{} =", name)).monospace().strong());
                            ui.label(egui::RichText::new(format!("{}", val)).monospace());
                        });
                    }
                    for (name, f) in &self.env.fns {
                        let params = f.params.join(", ");
                        ui.label(egui::RichText::new(
                            format!("{}({}) = {}", name, params, f.body)
                        ).monospace().italics());
                    }
                });
            });

        egui::Panel::right("fn_panel")
            .min_size(170.0)
            .max_size(200.0)
            .show_inside(ui, |ui| {
                ui.heading("Functions");
                ui.separator();
                function_palette(ui, &mut self.input);
            });

        egui::Panel::bottom("input_panel")
            .min_size(50.0)
            .show_inside(ui, |ui| {
                ui.separator();
                ui.horizontal(|ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.input)
                            .desired_width(f32::INFINITY)
                            .hint_text("enter expression, fn f(x) = …, or x = …")
                            .font(egui::TextStyle::Monospace),
                    );
                    let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("=").clicked() || enter {
                        self.evaluate();
                        resp.request_focus();
                    }
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("History");
            ui.separator();
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for (expr_str, result) in &self.history {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(expr_str).monospace());
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(egui::RichText::new(result).monospace().strong());
                                },
                            );
                        });
                    }
                });
        });
    }
}
