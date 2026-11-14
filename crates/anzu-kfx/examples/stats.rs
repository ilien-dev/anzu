//! Verificación manual: qué estructuras usa de verdad un libro. Solo cuenta; no imprime texto.
use std::collections::BTreeMap;

use anzu_kfx::container::{self, FragmentValue};
use anzu_kfx::ion::Ion;
use anzu_kfx::symbols::SymbolTable;

fn name(symbols: &SymbolTable, sid: u32) -> String {
    symbols.name(sid).map_or_else(|| format!("${sid}"), str::to_string)
}

#[derive(Default)]
struct Stats {
    node_types: BTreeMap<String, usize>,
    node_fields: BTreeMap<String, usize>,
    event_fields: BTreeMap<String, usize>,
    style_fields: BTreeMap<String, usize>,
    units: BTreeMap<String, usize>,
    symbol_values: BTreeMap<String, BTreeMap<String, usize>>,
    first_event: Option<String>,
}

fn walk_nodes(v: &Ion, symbols: &SymbolTable, stats: &mut Stats) {
    match v.unannotated() {
        Ion::List(items) | Ion::Sexp(items) => items.iter().for_each(|i| walk_nodes(i, symbols, stats)),
        Ion::Struct(fields) => {
            let is_node = fields.iter().any(|(sid, _)| *sid == 159 || *sid == 155);
            for (sid, value) in fields {
                if is_node {
                    *stats.node_fields.entry(name(symbols, *sid)).or_default() += 1;
                }
                if *sid == 159
                    && let Ion::Symbol(t) = value.unannotated()
                {
                    *stats.node_types.entry(name(symbols, *t)).or_default() += 1;
                }
                if *sid == 142
                    && let Ion::List(events) = value.unannotated()
                {
                    for event in events {
                        if let Ion::Struct(ef) = event.unannotated() {
                            for (esid, _) in ef {
                                *stats.event_fields.entry(name(symbols, *esid)).or_default() += 1;
                            }
                            if stats.first_event.is_none() {
                                stats.first_event = Some(describe(event, symbols));
                            }
                        }
                    }
                }
                walk_nodes(value, symbols, stats);
            }
        }
        _ => {}
    }
}

/// Un struct en una línea, con las cadenas sustituidas por su longitud.
fn describe(v: &Ion, symbols: &SymbolTable) -> String {
    match v.unannotated() {
        Ion::Struct(fields) => {
            let parts: Vec<String> =
                fields.iter().map(|(sid, value)| format!("{}: {}", name(symbols, *sid), describe(value, symbols))).collect();
            format!("{{{}}}", parts.join(", "))
        }
        Ion::List(items) => format!("[{} elementos]", items.len()),
        Ion::Symbol(s) => format!("'{}'", name(symbols, *s)),
        Ion::String(s) => format!("<texto de {}>", s.chars().count()),
        other => format!("{other:?}"),
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("uso: stats <archivo.kfx-zip>");
    let fragments = container::open(std::path::Path::new(&path)).expect("no se pudo abrir");
    let symbols = &fragments.symbols;
    let mut stats = Stats::default();

    for f in fragments.of_type("$259") {
        if let FragmentValue::Ion(v) = &f.value {
            walk_nodes(v, symbols, &mut stats);
        }
    }
    for f in fragments.of_type("$157") {
        let FragmentValue::Ion(v) = &f.value else { continue };
        let Ion::Struct(fields) = v.unannotated() else { continue };
        for (sid, value) in fields {
            let key = name(symbols, *sid);
            *stats.style_fields.entry(key.clone()).or_default() += 1;
            match value.unannotated() {
                Ion::Symbol(s) => {
                    *stats.symbol_values.entry(key).or_default().entry(name(symbols, *s)).or_default() += 1;
                }
                Ion::Struct(_) => {
                    if let Some(Ion::Symbol(u)) = value.field(306).map(Ion::unannotated) {
                        *stats.units.entry(format!("{key} en {}", name(symbols, *u))).or_default() += 1;
                    }
                }
                _ => {}
            }
        }
    }

    let print = |title: &str, map: &BTreeMap<String, usize>| {
        println!("-- {title}");
        let line: Vec<String> = map.iter().map(|(k, n)| format!("{k}={n}")).collect();
        println!("   {}", line.join("  "));
    };
    print("tipos de nodo ($159)", &stats.node_types);
    print("campos de nodo", &stats.node_fields);
    print("campos de eventos de estilo ($142)", &stats.event_fields);
    println!("-- primer evento: {}", stats.first_event.unwrap_or_else(|| "ninguno".into()));
    print("propiedades de estilo ($157)", &stats.style_fields);
    print("unidades", &stats.units);
    println!("-- valores simbólicos de estilo");
    for (key, values) in &stats.symbol_values {
        let line: Vec<String> = values.iter().map(|(k, n)| format!("{k}={n}")).collect();
        println!("   {key}: {}", line.join("  "));
    }
}
