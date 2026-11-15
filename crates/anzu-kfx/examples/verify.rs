//! Manual check: internal integrity of a generated EPUB. It does not print the book's text.
//! It checks for well-formed XML, the manifest, the reading order, and that every link,
//! image and table of contents entry points at something that exists.
use std::collections::{HashMap, HashSet};
use std::io::Read;

fn resolve(base: &str, href: &str) -> String {
    let mut parts: Vec<&str> = base.split('/').collect();
    parts.pop();
    for piece in href.split('/') {
        match piece {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            other => parts.push(other),
        }
    }
    parts.join("/")
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: verify <book.epub>");
    let mut archive =
        zip::ZipArchive::new(std::fs::File::open(&path).expect("could not open the file")).expect("invalid zip");
    let mut problems: Vec<String> = Vec::new();
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let name = entry.name().unwrap().into_owned();
        if i == 0 && (name != "mimetype" || entry.compression() != zip::CompressionMethod::Stored) {
            problems.push("the first entry is not an uncompressed `mimetype`".into());
        }
        let mut data = Vec::new();
        entry.read_to_end(&mut data).unwrap();
        order.push(name.clone());
        files.insert(name, data);
    }

    let is_xml = |n: &str| [".xhtml", ".opf", ".ncx", ".xml"].iter().any(|e| n.ends_with(e));
    let texts: HashMap<&str, &str> =
        order.iter().filter(|n| is_xml(n)).map(|n| (n.as_str(), std::str::from_utf8(&files[n]).unwrap_or(""))).collect();
    let mut docs: HashMap<&str, roxmltree::Document> = HashMap::new();
    for (name, text) in &texts {
        match roxmltree::Document::parse(text) {
            Ok(doc) => {
                docs.insert(name, doc);
            }
            Err(e) => problems.push(format!("{name}: XML is not well-formed ({e})")),
        }
    }

    // Identifiers per document, and duplicates.
    let mut ids: HashMap<&str, HashSet<&str>> = HashMap::new();
    for (name, doc) in &docs {
        let set = ids.entry(name).or_default();
        for node in doc.descendants() {
            if let Some(id) = node.attribute("id")
                && !set.insert(id)
            {
                problems.push(format!("{name}: duplicate identifier `{id}`"));
            }
        }
    }

    let opf_name = "OEBPS/content.opf";
    let mut manifest: HashMap<&str, String> = HashMap::new();
    let mut spine = 0;
    if let Some(opf) = docs.get(opf_name) {
        for item in opf.descendants().filter(|n| n.has_tag_name("item")) {
            let (Some(id), Some(href)) = (item.attribute("id"), item.attribute("href")) else { continue };
            let full = resolve(opf_name, href);
            if !files.contains_key(&full) {
                problems.push(format!("manifest: file {full} is missing"));
            }
            manifest.insert(id, full);
        }
        for itemref in opf.descendants().filter(|n| n.has_tag_name("itemref")) {
            spine += 1;
            if !itemref.attribute("idref").is_some_and(|r| manifest.contains_key(r)) {
                problems.push("reading order: reference to an item that is not in the manifest".into());
            }
        }
        let listed: HashSet<&String> = manifest.values().collect();
        for name in order.iter().filter(|n| n.starts_with("OEBPS/") && n.as_str() != opf_name) {
            if !listed.contains(name) {
                problems.push(format!("{name} is not in the manifest"));
            }
        }
    } else {
        problems.push("OEBPS/content.opf is missing".into());
    }

    // Links, images and table of contents.
    let (mut links, mut external, mut images, mut words) = (0, 0, 0, 0usize);
    for (name, doc) in &docs {
        for node in doc.descendants() {
            if node.is_text() && name.starts_with("OEBPS/text/") {
                words += node.text().unwrap_or("").split_whitespace().count();
            }
            let target = match (node.tag_name().name(), node.attribute("href"), node.attribute("src")) {
                ("a", Some(href), _) => Some(href),
                ("content", _, Some(src)) => Some(src),
                ("img", _, Some(src)) => {
                    images += 1;
                    Some(src)
                }
                _ => None,
            };
            let Some(target) = target else { continue };
            if target.contains("://") || target.starts_with("mailto:") {
                external += 1;
                continue;
            }
            links += 1;
            let (file, fragment) = target.split_once('#').map_or((target, None), |(f, a)| (f, Some(a)));
            let full = resolve(name, file);
            if !files.contains_key(&full) {
                problems.push(format!("{name}: link to a file that does not exist ({target})"));
            } else if let Some(fragment) = fragment
                && !ids.get(full.as_str()).is_some_and(|set| set.contains(fragment))
            {
                problems.push(format!("{name}: link to an anchor that does not exist ({target})"));
            }
        }
    }

    println!("Entries:             {}", order.len());
    println!("XML documents:       {} parsed", docs.len());
    println!("Manifest:            {} items, {} in the reading order", manifest.len(), spine);
    println!("Internal references: {links} checked ({images} images), {external} external");
    println!("Words in the text:   {words}");
    println!("Problems:            {}", problems.len());
    for p in problems.iter().take(20) {
        println!("  - {p}");
    }
    if !problems.is_empty() {
        std::process::exit(1);
    }
}
