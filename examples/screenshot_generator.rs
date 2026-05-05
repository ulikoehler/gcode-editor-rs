use eframe::egui;
use gcode_editor::{show_editor, EditorEvent, EditorState, SyntaxColors};
use std::env;
use std::fs;
use std::path::Path;

struct ScreenshotApp {
    content: String,
    state: EditorState,
    colors: SyntaxColors,
    window_title: String,
}

impl ScreenshotApp {
    fn new(file_path: &str) -> Self {
        let content = if Path::new(file_path).exists() {
            fs::read_to_string(file_path).unwrap_or_else(|_| {
                eprintln!("Failed to read file: {}", file_path);
                String::from("; Failed to load file\n")
            })
        } else {
            eprintln!("File not found: {}", file_path);
            String::from("; File not found\n")
        };

        let mut state = EditorState::default();
        state.show_line_numbers = true;
        state.active_line = Some(1);
        state.show_active_line_bg = true;

        let filename = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        Self {
            content,
            state,
            colors: SyntaxColors::default(),
            window_title: format!("GCode Editor - {}", filename),
        }
    }
}

impl eframe::App for ScreenshotApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading(&self.window_title);
            let events = show_editor(ui, &mut self.content, &mut self.state, &self.colors, 14.0);
            for evt in events {
                match evt {
                    EditorEvent::ContentChanged(c) => {
                        println!("Editor changed: {} bytes", c.new_content.len())
                    }
                    EditorEvent::ActiveLineChanged { old, new } => {
                        println!("Active line changed: {:?} -> {:?}", old, new)
                    }
                    EditorEvent::SelectionChanged { old, new } => {
                        println!("Selection changed: {:?} -> {:?}", old, new)
                    }
                }
            }
        });

        // Request continuous repaint to ensure the window is fully rendered
        ui.ctx().request_repaint();
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check for --screenshot flag to automatically generate screenshot
    if args.len() > 1 && args[1] == "--screenshot-o100" {
        generate_o100_screenshot();
        return;
    }

    if args.len() < 2 {
        eprintln!("Usage: {} <gcode_file>", args[0]);
        eprintln!("Example: {} examples/gcode/axis_examples.gcode", args[0]);
        eprintln!(
            "       {} --screenshot-o100  (generate screenshot of O100 example)",
            args[0]
        );
        std::process::exit(1);
    }

    let file_path = &args[1];

    let mut options = eframe::NativeOptions::default();
    options.viewport.inner_size = Some(egui::vec2(800.0, 600.0));

    let app = ScreenshotApp::new(file_path);
    let window_title = app.window_title.clone();

    if let Err(e) = eframe::run_native(&window_title, options, Box::new(|_cc| Ok(Box::new(app)))) {
        eprintln!("Error running application: {}", e);
        std::process::exit(1);
    }
}

fn generate_o100_screenshot() {
    let file_path = "examples/gcode/ocodes_examples.gcode";

    let mut options = eframe::NativeOptions::default();
    options.viewport.inner_size = Some(egui::vec2(800.0, 600.0));
    options.persist_window = false; // Don't restore window state

    let app = ScreenshotApp::new(file_path);
    let window_title = app.window_title.clone();

    // Run the app and capture screenshot after a few frames
    if let Err(e) = eframe::run_native(&window_title, options, Box::new(|_cc| Ok(Box::new(app)))) {
        eprintln!("Error running application: {}", e);
        std::process::exit(1);
    }
}
