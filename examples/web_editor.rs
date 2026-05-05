//! Web interface example for the GCode editor using egui.
//!
//! This example demonstrates how to use the gcode_editor library in a web browser
//! context using egui's web backend (eframe).
//!
//! To run this example as a web application:
//!
//! 1. Install the web target: rustup target add wasm32-unknown-unknown
//! 2. Install trunk: cargo install trunk
//! 3. Run: trunk serve --open --example web_editor
//!
//! The editor will open in your default web browser.

use eframe::egui;
use gcode_editor::{show_editor, EditorEvent, EditorState, SyntaxColors};

struct WebEditorApp {
    content: String,
    state: EditorState,
    colors: SyntaxColors,
    // Additional web-specific UI state
    show_stats: bool,
    last_saved: Option<String>,
}

impl Default for WebEditorApp {
    fn default() -> Self {
        let mut state = EditorState::default();
        state.show_line_numbers = true;
        state.show_active_line_bg = true;
        state.active_line = Some(1);

        // Sample GCode content
        let content = r#"; Example GCode program
; Simple square pattern
G21 ; Set units to millimeters
G90 ; Absolute positioning
G28 ; Home all axes

; Start at origin
G0 X0 Y0 Z5

; Draw a square
G1 X50 Y0 F1500
G1 X50 Y50
G1 X0 Y50
G1 X0 Y0

; Retract and finish
G0 Z10
M2 ; End of program
"#
        .to_string();

        Self {
            content,
            state,
            colors: SyntaxColors::default(),
            show_stats: true,
            last_saved: None,
        }
    }
}

impl eframe::App for WebEditorApp {
    fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Configure dark theme for web
        ctx.set_visuals(egui::Visuals::dark());

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🌐 GCode Web Editor");
            ui.label("Edit GCode directly in your browser with syntax highlighting");
            ui.add_space(10.0);

            // Toolbar
            ui.horizontal(|ui| {
                if ui.button("📋 Copy").clicked() {
                    // Copy to clipboard (web-compatible)
                    let _ = ctx.copy_text(self.content.clone());
                }

                if ui.button("🗑️ Clear").clicked() {
                    self.content.clear();
                }

                if ui.button("📊 Toggle Stats").clicked() {
                    self.show_stats = !self.show_stats;
                }

                ui.separator();

                // Show line count
                let line_count = self.content.lines().count();
                ui.label(format!("Lines: {}", line_count));
            });

            ui.add_space(10.0);

            // Main editor
            let events = show_editor(ui, &mut self.content, &mut self.state, &self.colors, 14.0);

            // Handle editor events
            for evt in events {
                match evt {
                    EditorEvent::ContentChanged(_c) => {
                        // In a real app, you might auto-save here
                        self.last_saved = Some("Modified".to_string());
                    }
                    EditorEvent::ActiveLineChanged { old: _, new: _ } => {
                        // Could be used for debugging or external coordination
                    }
                    EditorEvent::SelectionChanged { old: _, new: _ } => {
                        // Could be used for context menus or operations
                    }
                }
            }

            ui.add_space(10.0);

            // Statistics panel
            if self.show_stats {
                ui.separator();
                ui.collapsing("📊 Statistics", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Characters:");
                        ui.label(self.content.len().to_string());
                    });
                    ui.horizontal(|ui| {
                        ui.label("Lines:");
                        ui.label(self.content.lines().count().to_string());
                    });
                    ui.horizontal(|ui| {
                        ui.label("G-commands:");
                        let g_count = self.content.matches('G').count();
                        ui.label(g_count.to_string());
                    });
                    ui.horizontal(|ui| {
                        ui.label("M-commands:");
                        let m_count = self.content.matches('M').count();
                        ui.label(m_count.to_string());
                    });
                    if let Some(ref status) = self.last_saved {
                        ui.label(format!("Status: {}", status));
                    }
                });
            }

            ui.add_space(10.0);
            ui.separator();
            ui.label("💡 Tips:");
            ui.label("- Click line numbers to navigate to specific lines");
            ui.label("- Use Ctrl+F to search (not implemented in this example)");
            ui.label("- The editor supports syntax highlighting for G, M, and axis commands");
        });
    }
}

// Web-specific main function
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect console.log to wasm console
    console_error_panic_hook::set_once();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // This ID must match the HTML canvas element
                web_options,
                Box::new(|_cc| {
                    // Setup web-specific context
                    Ok(Box::new(WebEditorApp::default()))
                }),
            )
            .await
            .expect("failed to start eframe");
    });
}

// Native fallback for testing
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "GCode Web Editor (Native)",
        options,
        Box::new(|_cc| Ok(Box::new(WebEditorApp::default()))),
    );
}
