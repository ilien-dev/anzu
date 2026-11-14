//! Verificación manual: resumen del modelo de un libro. No imprime su texto.
use anzu_kfx::book::{self, Block, TocEntry};
use anzu_kfx::container;

fn count_toc(entries: &[TocEntry]) -> (usize, usize) {
    let mut total = 0;
    let mut depth = 0;
    for e in entries {
        let (n, d) = count_toc(&e.children);
        total += 1 + n;
        depth = depth.max(1 + d);
    }
    (total, depth)
}

fn main() {
    let path = std::env::args().nth(1).expect("uso: outline <archivo.kfx-zip>");
    let fragments = container::open(std::path::Path::new(&path)).expect("no se pudo abrir");
    let book = match book::build(&fragments) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ERROR: {e}");
            std::process::exit(1);
        }
    };

    let (mut headings, mut paragraphs, mut images, mut words, mut links, mut anchors) = (0, 0, 0, 0usize, 0, 0);
    let (mut bold, mut italic) = (0, 0);
    let mut empty_chapters = 0;
    for chapter in &book.chapters {
        if chapter.blocks.is_empty() {
            empty_chapters += 1;
        }
        for block in &chapter.blocks {
            let (spans, anchor) = match block {
                Block::Heading { spans, anchor, .. } => {
                    headings += 1;
                    (spans.as_slice(), anchor)
                }
                Block::Paragraph { spans, anchor, .. } => {
                    paragraphs += 1;
                    (spans.as_slice(), anchor)
                }
                Block::Image { anchor, .. } => {
                    images += 1;
                    (&[][..], anchor)
                }
            };
            anchors += usize::from(anchor.is_some());
            for span in spans {
                words += span.text.split_whitespace().count();
                links += usize::from(span.link.is_some());
                bold += usize::from(span.bold);
                italic += usize::from(span.italic);
            }
        }
    }
    let (toc_entries, toc_depth) = count_toc(&book.toc);

    println!("Título:     {}", book.meta.title);
    println!("Autores:    {}", book.meta.authors.join("; "));
    println!("Idioma:     {}", book.meta.language);
    println!("ASIN:       {}", book.meta.asin.as_deref().unwrap_or("-"));
    println!("Editorial:  {}", book.meta.publisher.as_deref().unwrap_or("-"));
    println!("Portada:    {}", book.cover.as_deref().unwrap_or("-"));
    println!("Capítulos:  {} ({} vacíos)", book.chapters.len(), empty_chapters);
    println!("Bloques:    {headings} encabezados, {paragraphs} párrafos, {images} imágenes");
    println!("Palabras:   {words}");
    println!("Formato:    {bold} tramos en negrita, {italic} en cursiva, {links} enlaces, {anchors} anclas");
    println!("Índice:     {toc_entries} entradas, profundidad {toc_depth}");
    println!("Imágenes:   {}", book.images.iter().map(|i| format!("{} ({} bytes)", i.name, i.data.len())).collect::<Vec<_>>().join(", "));
    println!("Avisos:     {}", book.warnings.len());
    for w in book.warnings.iter().take(15) {
        println!("  - {w}");
    }
}
