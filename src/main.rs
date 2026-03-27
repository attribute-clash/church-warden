use church_warden::{Database, gui::ChurchWardenApp};

fn main() -> eframe::Result<()> {
    let db = Database::from_path("church_warden.db").expect("db init");
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Church Warden",
        options,
        Box::new(move |_cc| Ok(Box::new(ChurchWardenApp::new(db)))),
    )
}
