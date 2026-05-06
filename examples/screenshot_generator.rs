use egui_kittest::Harness;
use gcode_editor::{show_editor, EditorState, SyntaxColors};
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <gcode_file> [output.png]", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = if args.len() > 2 {
        args[2].clone()
    } else {
        let stem = Path::new(input_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        format!("{}.png", stem)
    };

    let content = fs::read_to_string(input_path).unwrap_or_else(|e| {
        eprintln!("Failed to read {}: {}", input_path, e);
        std::process::exit(1);
    });

    let mut content_mut = content;
    let mut state = EditorState::default();
    state.show_line_numbers = true;
    state.active_line = Some(1);
    state.show_active_line_bg = true;
    let colors = SyntaxColors::default();

    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .with_theme(egui::Theme::Dark)
        .build_ui(|ui| {
            show_editor(ui, &mut content_mut, &mut state, &colors, 14.0);
        });

    harness.run();

    let image = harness.render().expect("Failed to render");
    image.save(&output_path).expect("Failed to save PNG");

    println!("Screenshot saved to: {}", output_path);
}
