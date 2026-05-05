use eframe::egui;
use gcode_editor::{show_editor, EditorEvent, EditorState, SyntaxColors};

struct SimpleEditorApp {
    content: String,
    state: EditorState,
    colors: SyntaxColors,
}

impl Default for SimpleEditorApp {
    fn default() -> Self {
        let mut state = EditorState::default();
        state.show_line_numbers = true;
        // Active line set to line 1 in the example
        state.active_line = Some(1);
        // Show an active-line background in the example
        state.show_active_line_bg = true;

        Self {
            content: "G0 X0 Y0\nG1 X100 Y100 F1500\n".to_string(),
            state,
            colors: SyntaxColors::default(),
        }
    }
}

impl eframe::App for SimpleEditorApp {
    fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("GCode Editor Example");
            let events = show_editor(ui, &mut self.content, &mut self.state, &self.colors, 14.0);
            for evt in events {
                match evt {
                    EditorEvent::ContentChanged(c) => println!("Editor changed: {} bytes", c.new_content.len()),
                    EditorEvent::ActiveLineChanged { old, new } => println!("Active line changed: {:?} -> {:?}", old, new),
                    EditorEvent::SelectionChanged { old, new } => println!("Selection changed: {:?} -> {:?}", old, new),
                }
            }

            ui.separator();
            ui.label("Use the Error Demo example to see syntax error highlighting (hover errors to view tooltips).");
        });
    }
}

fn main() {
    let options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "GCode Editor Example",
        options,
        Box::new(|_cc| Ok(Box::new(SimpleEditorApp::default()))),
    );
}
