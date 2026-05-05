use eframe::egui;
use gcode_editor::{show_editor, EditorEvent, EditorState, SyntaxColors};

struct ErrorDemoApp {
    content: String,
    state: EditorState,
    colors: SyntaxColors,
}

impl Default for ErrorDemoApp {
    fn default() -> Self {
        let mut state = EditorState::default();
        state.show_line_numbers = true;
        state.show_active_line_bg = true;

        // Example that contains an obvious syntax error: a non-numeric axis value "Xabc"
        let content = "G0 X0 Y0\nG1 Xabc Y100 ; invalid X value\nG1 X50 Y50\n".to_string();

        // Mark the offending token "Xabc" as an error with a tooltip
        // find byte offsets for the substring
        if let Some(start) = content.find("Xabc") {
            let end = start + "Xabc".len();
            state.add_error_range_bytes_with_tooltip(
                start,
                end,
                "Invalid numeric value for axis X",
            );
        }

        Self {
            content,
            state,
            colors: SyntaxColors::default(),
        }
    }
}

impl eframe::App for ErrorDemoApp {
    fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("GCode Syntax Error Demo");

            let events = show_editor(ui, &mut self.content, &mut self.state, &self.colors, 14.0);
            for evt in events {
                match evt {
                    EditorEvent::ContentChanged(c) => eprintln!("Editor changed: {} bytes", c.new_content.len()),
                    EditorEvent::ActiveLineChanged { old, new } => eprintln!("Active line: {:?} -> {:?}", old, new),
                    EditorEvent::SelectionChanged { old, new } => eprintln!("Selection changed: {:?} -> {:?}", old, new),
                }
            }

            ui.separator();
            ui.label("This demo pre-marks the token 'Xabc' as a syntax error and shows a tooltip when hovered.");
        });
    }
}

fn main() {
    let options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "GCode Editor Error Demo",
        options,
        Box::new(|_cc| Ok(Box::new(ErrorDemoApp::default()))),
    );
}
