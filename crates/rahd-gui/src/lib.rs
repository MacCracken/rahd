//! Rahd GUI — Desktop calendar with day/week/month views
//!
//! Built with egui/eframe following the AGNOS desktop app pattern.

mod app;
mod notifications;
#[allow(dead_code)]
mod theme;
mod views;

pub use app::RahdApp;

use rahd_store::EventStore;
use std::sync::{Arc, Mutex};

/// Launch the GUI window. Blocks until closed.
pub fn run(store: Arc<Mutex<EventStore>>) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Rahd — Calendar & Contacts")
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([640.0, 420.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Rahd",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(RahdApp::new(store)))
        }),
    )
}
