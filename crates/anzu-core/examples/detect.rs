//! Manual check: what Anzu sees on this machine.
use anzu_core::{kindle, library, settings};

fn main() {
    let data = settings::data_dir();
    let s = settings::Settings::load(&data);
    match kindle::detect() {
        Some(k) => {
            println!("Kindle {} at {}", k.version, k.content_dir.display());
            println!("Running: {}", kindle::is_running());
            for b in library::scan(&k.content_dir, &data, &s) {
                println!("{} {:?} {} bytes {:?}", b.asin, b.state, b.size, b.title);
            }
        }
        None => println!("Kindle is not installed"),
    }
}
