//! Verificación manual: recuento de fragmentos por tipo de un KFX sin DRM.
use std::collections::BTreeMap;

use seshat_kfx::container::{self, FragmentValue};

fn main() {
    let path = std::env::args().nth(1).expect("uso: dump <archivo.kfx-zip>");
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
    println!("{} fragmentos", fragments.items.len());
    for (ftype, (n, raw)) in counts {
        if raw > 0 {
            println!("{ftype:>6}  {n:>5}  ({raw} bytes en bruto)");
        } else {
            println!("{ftype:>6}  {n:>5}");
        }
    }
}
