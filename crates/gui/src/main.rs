use eframe::egui;
use opencalc_core::{parse, simplify, eval, Context};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("opencalc")
            .with_inner_size([480.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native("opencalc", options, Box::new(|_| Ok(Box::new(App::new()))))
}

struct App {
    input:   String,
    history: Vec<(String, String)>,
    ctx:     Context,
}

impl App {
    fn new() -> Self {
        App { input: String::new(), history: Vec::new(), ctx: Context::new() }
    }

    fn evaluate(&mut self) {
        let input = self.input.trim().to_string();
        if input.is_empty() { return; }

        // Variable assignment: name = expr
        if let Some((name, rhs)) = input.split_once('=') {
            let name = name.trim();
            let rhs  = rhs.trim();
            if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                let res = parse(rhs).map(simplify)
                    .and_then(|e| eval(&e, &self.ctx).map_err(Into::into));
                let display = match res {
                    Ok(v)  => { self.ctx.set(name, v); format!("{} = {}", name, v) }
                    Err(e) => format!("error: {}", e),
                };
                self.history.push((input, display));
                self.input.clear();
                return;
            }
        }

        let display = match parse(&input).map(simplify) {
            Ok(expr) => match eval(&expr, &self.ctx) {
                Ok(v)  => v.to_string(),
                Err(_) => format!("{}", expr),
            },
            Err(e) => format!("error: {}", e),
        };

        self.history.push((input, display));
        self.input.clear();
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("opencalc");
        ui.separator();

        let available = ui.available_height() - 60.0;
        egui::ScrollArea::vertical()
            .max_height(available)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for (expr, result) in &self.history {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(expr).monospace());
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| { ui.label(egui::RichText::new(result).monospace().strong()); },
                        );
                    });
                }
            });

        ui.separator();

        ui.horizontal(|ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.input)
                    .desired_width(f32::INFINITY)
                    .hint_text("enter expression…")
                    .font(egui::TextStyle::Monospace),
            );
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button("=").clicked() || enter {
                self.evaluate();
                resp.request_focus();
            }
        });
    }
}
