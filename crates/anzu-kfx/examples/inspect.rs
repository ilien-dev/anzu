//! Manual check: structure of the fragments of one type, without dumping the book's text.
//! usage: inspect <file> <type, e.g. $260> [how many] [depth]
use anzu_kfx::container::{self, FragmentValue};
use anzu_kfx::ion::Ion;
use anzu_kfx::symbols::SymbolTable;

const MAX_STRING: usize = 48;

fn name(symbols: &SymbolTable, sid: u32) -> String {
    symbols.name(sid).map_or_else(|| format!("${sid}"), str::to_string)
}

fn show(v: &Ion, symbols: &SymbolTable, indent: usize, depth: usize, mask_all: bool) {
    let pad = "  ".repeat(indent);
    match v {
        Ion::Struct(fields) => {
            if depth == 0 {
                println!("{pad}{{…{} fields}}", fields.len());
                return;
            }
            for (sid, value) in fields {
                match value.unannotated() {
                    Ion::Struct(_) | Ion::List(_) | Ion::Sexp(_) => {
                        println!("{pad}{}:", name(symbols, *sid));
                        show(value, symbols, indent + 1, depth - 1, mask_all);
                    }
                    _ => println!("{pad}{}: {}", name(symbols, *sid), scalar(value, symbols, mask_all)),
                }
            }
        }
        Ion::List(items) | Ion::Sexp(items) => {
            if depth == 0 {
                println!("{pad}[…{} items]", items.len());
                return;
            }
            let shown = items.len().min(4);
            for item in &items[..shown] {
                match item.unannotated() {
                    Ion::Struct(_) | Ion::List(_) | Ion::Sexp(_) => {
                        println!("{pad}-");
                        show(item, symbols, indent + 1, depth - 1, mask_all);
                    }
                    _ => println!("{pad}- {}", scalar(item, symbols, mask_all)),
                }
            }
            if items.len() > shown {
                println!("{pad}… and {} more", items.len() - shown);
            }
        }
        Ion::Annotated(annots, inner) => {
            let names: Vec<String> = annots.iter().map(|a| name(symbols, *a)).collect();
            println!("{pad}@{}", names.join(","));
            show(inner, symbols, indent, depth, mask_all);
        }
        other => println!("{pad}{}", scalar(other, symbols, mask_all)),
    }
}

fn scalar(v: &Ion, symbols: &SymbolTable, mask_all: bool) -> String {
    match v {
        Ion::Symbol(s) => format!("'{}'", name(symbols, *s)),
        Ion::String(s) if mask_all || s.chars().count() > MAX_STRING => format!("<text of {} characters>", s.chars().count()),
        Ion::String(s) => format!("{s:?}"),
        Ion::Blob(b) | Ion::Clob(b) => format!("<{} bytes>", b.len()),
        Ion::Annotated(annots, inner) => {
            let names: Vec<String> = annots.iter().map(|a| name(symbols, *a)).collect();
            format!("@{} {}", names.join(","), scalar(inner, symbols, mask_all))
        }
        other => format!("{other:?}"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("usage: inspect <file> <type> [how many] [depth]");
    let ftype = args.get(2).expect("the type is missing");
    let count: usize = args.get(3).and_then(|n| n.parse().ok()).unwrap_or(1);
    let depth: usize = args.get(4).and_then(|n| n.parse().ok()).unwrap_or(6);

    let fragments = container::open(std::path::Path::new(path)).expect("could not open the file");
    // Type $145 holds the book's prose: it is always masked.
    let mask_all = ftype == "$145";
    for f in fragments.of_type(ftype).take(count) {
        println!("== {} {}", f.ftype, f.fid);
        match &f.value {
            FragmentValue::Ion(v) => show(v, &fragments.symbols, 1, depth, mask_all),
            FragmentValue::Raw(b) => println!("  <{} raw bytes>", b.len()),
        }
    }
}
