//! Verificación manual: qué ve Anzu en este equipo.
use anzu_core::{kindle, library, settings};

fn main() {
    let data = settings::data_dir();
    let s = settings::Settings::load(&data);
    match kindle::detect() {
        Some(k) => {
            println!("Kindle {} en {}", k.version, k.content_dir.display());
            println!("En ejecución: {}", kindle::is_running());
            for b in library::scan(&k.content_dir, &data, &s) {
                println!("{} {:?} {} bytes", b.asin, b.state, b.size);
            }
        }
        None => println!("Kindle no instalada"),
    }
}
