//! brain-map's page: the graph, the reader and the chrome, in one window.
//!
//! Half of this crate never touches egui — the simulation, the note renderer, link
//! resolution, the keymap, the legend filter, the theme table — and that half is where
//! the tests live. The other half draws.

mod app;
mod chrome;
mod emoji;
mod filter;
mod keys;
mod links;
mod markdown;
mod reader;
mod render;
mod settings;
mod sim;
mod state;
mod theme;
mod tree;

use brain_map_model::Source;
use std::path::PathBuf;
use std::sync::Arc;

/// Open the window on a vault, or on the picker when there is none yet. The error is a
/// string so that nothing of egui reaches the scanner side.
pub fn run(source: Arc<dyn Source>, vault: Option<PathBuf>) -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("brain-map")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 400.0]),
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "brain-map",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, source, vault)))),
    )
    .map_err(|e| e.to_string())
}
