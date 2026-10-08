#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! AutoKey - tape des touches ou du texte à une heure précise, dans la fenêtre de ton choix (Windows).
mod engine;
mod keys;
mod model;
mod ui;
mod worker;

use eframe::egui;

fn load_icon() -> egui::IconData {
    let img = image::load_from_memory(include_bytes!("../assets/autokey.png")).expect("icône").to_rgba8();
    let (width, height) = img.dimensions();
    egui::IconData { rgba: img.into_raw(), width, height }
}

const TITLE: &str = "AutoKey – touche à heure précise";

fn main() -> eframe::Result {
    // une seule instance (sauf pour les tests automatiques, qui utilisent leurs propres réglages)
    if std::env::var_os("AUTOKEY_SETTINGS").is_none() && engine::another_instance_running(TITLE) {
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(TITLE)
            .with_inner_size([1100.0, 640.0])
            .with_position([60.0, 20.0])
            .with_min_inner_size([1000.0, 460.0])
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native("AutoKey", options, Box::new(|cc| Ok(Box::new(ui::App::new(cc)))))
}
