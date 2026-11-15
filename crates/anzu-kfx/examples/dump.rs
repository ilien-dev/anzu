//! Manual check: fragment count by type of a DRM-free KFX.
use std::collections::BTreeMap;

use anzu_kfx::container::{self, FragmentValue};

fn main() {
    let path = std::env::args().nth(1).expect("usage: dump <file.kfx-zip>");
    let fragments = match container::open(std::path::Path::new(&path)) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("ERROR: {e}");
            std::process::exit(1);
        }
    };
    let mut counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for f in &fragments.items {
        let entry = counts.entry(f.ftype.as_str()).or_default();
        entry.0 += 1;
        if let FragmentValue::Raw(bytes) = &f.value {
            entry.1 += bytes.len();
        }
    }
    println!("{} fragments", fragments.items.len());
    for (ftype, (n, raw)) in counts {
        if raw > 0 {
            println!("{ftype:>6}  {n:>5}  ({raw} raw bytes)");
        } else {
            println!("{ftype:>6}  {n:>5}");
        }
    }
}
