//! Escritor de EPUB 3 a partir del modelo del libro.
//!
//! Estructura del archivo:
//!
//! ```text
//! mimetype                  primero y sin comprimir
//! META-INF/container.xml
//! OEBPS/content.opf         metadatos, manifiesto y orden de lectura
//! OEBPS/nav.xhtml           índice (EPUB 3)
//! OEBPS/toc.ncx             índice (lectores EPUB 2)
//! OEBPS/style.css
//! OEBPS/text/<id>.xhtml     un archivo por capítulo
//! OEBPS/images/<nombre>
//! ```

use std::io::Write;
use std::path::Path;

use crate::Error;
use crate::book::{Align, Block, Book, Chapter, Length, Span, Style, TocEntry};

const MAX_NAME_CHARS: usize = 120;
const PLACEHOLDER_NAME: &str = "libro";
const ILLEGAL_IN_NAMES: &str = "<>:\"/\\|?*";
const RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1",
    "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

const BASE_CSS: &str = "\
body { margin: 0 5%; line-height: 1.4; }
p { margin: 0; }
h1, h2, h3, h4, h5, h6 { margin: 1.2em 0 0.6em; line-height: 1.2; }
img { max-width: 100%; height: auto; }
.img { margin: 1em 0; text-align: center; }
nav ol { list-style: none; padding-left: 1.2em; }
";

/// Un nombre de archivo (sin extensión) válido en Windows a partir de un título.
pub fn safe_file_name(title: &str) -> String {
    let replaced: String =
        title.chars().map(|c| if c.is_control() || ILLEGAL_IN_NAMES.contains(c) { '_' } else { c }).collect();
    let edge = |c: char| c == ' ' || c == '.';
    let trimmed = replaced.trim_matches(edge);
    let cut: String = trimmed.chars().take(MAX_NAME_CHARS).collect();
    let name = cut.trim_end_matches(edge);

    let stem = name.split('.').next().unwrap_or("").trim().to_ascii_uppercase();
    if name.is_empty() || RESERVED_NAMES.contains(&stem.as_str()) {
        return PLACEHOLDER_NAME.to_string();
    }
    name.to_string()
}

/// Segundos desde 1970 como `AAAA-MM-DDThh:mm:ssZ`.
pub fn format_utc(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    // Algoritmo de fecha civil a partir de días (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rest / 3_600, rest % 3_600 / 60, rest % 60)
}

/// Escapa texto para XML y descarta los caracteres que XML 1.0 no admite.
fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if c.is_control() || c == '\u{FFFE}' || c == '\u{FFFF}' => {}
            c => out.push(c),
        }
    }
    out
}

fn number(value: f64) -> String {
    let rounded = (value * 10_000.0).round() / 10_000.0;
    format!("{rounded}")
}

fn css_length(length: Length) -> String {
    match length {
        Length::Em(v) => format!("{}em", number(v)),
        Length::Percent(v) => format!("{}%", number(v)),
        Length::Rem(v) => format!("{}rem", number(v)),
        Length::Pt(v) => format!("{}pt", number(v)),
    }
}

fn css_declarations(style: &Style) -> String {
    let mut out = Vec::new();
    if let Some(align) = style.align {
        let value = match align {
            Align::Left => "left",
            Align::Right => "right",
            Align::Center => "center",
            Align::Justify => "justify",
        };
        out.push(format!("text-align: {value}"));
    }
    if let Some(l) = style.indent {
        out.push(format!("text-indent: {}", css_length(l)));
    }
    if let Some(l) = style.margin_top {
        out.push(format!("margin-top: {}", css_length(l)));
    }
    if let Some(l) = style.margin_bottom {
        out.push(format!("margin-bottom: {}", css_length(l)));
    }
    if let Some(l) = style.font_size {
        out.push(format!("font-size: {}", css_length(l)));
    }
    if style.bold {
        out.push("font-weight: bold".to_string());
    }
    if style.italic {
        out.push("font-style: italic".to_string());
    }
    out.join("; ")
}

/// Estilos distintos del libro; la posición de cada uno da nombre a su clase CSS.
struct Classes {
    styles: Vec<Style>,
}

impl Classes {
    fn collect(chapters: &[&Chapter]) -> Self {
        let mut styles: Vec<Style> = Vec::new();
        for chapter in chapters {
            for block in &chapter.blocks {
                let (Block::Heading { style, .. } | Block::Paragraph { style, .. }) = block else { continue };
                if *style != Style::default() && !styles.contains(style) {
                    styles.push(style.clone());
                }
            }
        }
        Classes { styles }
    }

    fn class_attr(&self, style: &Style) -> String {
        match self.styles.iter().position(|s| s == style) {
            Some(i) => format!(" class=\"s{}\"", i + 1),
            None => String::new(),
        }
    }

    fn css(&self) -> String {
        let mut out = BASE_CSS.to_string();
        for (i, style) in self.styles.iter().enumerate() {
            out.push_str(&format!(".s{} {{ {}; }}\n", i + 1, css_declarations(style)));
        }
        out
    }
}

/// `capítulo#ancla` como ruta relativa desde `prefix`; una URL se deja igual.
fn href(target: &str, prefix: &str) -> String {
    if target.contains("://") || target.starts_with("mailto:") {
        return target.to_string();
    }
    match target.split_once('#') {
        Some((chapter, anchor)) => format!("{prefix}{chapter}.xhtml#{anchor}"),
        None => format!("{prefix}{target}.xhtml"),
    }
}

fn spans_xhtml(spans: &[Span]) -> String {
    let mut out = String::new();
    for span in spans {
        let mut piece = esc(&span.text);
        if span.italic {
            piece = format!("<i>{piece}</i>");
        }
        if span.bold {
            piece = format!("<b>{piece}</b>");
        }
        if let Some(link) = &span.link {
            piece = format!("<a href=\"{}\">{piece}</a>", esc(&href(link, "")));
        }
        out.push_str(&piece);
    }
    out
}

fn id_attr(anchor: &Option<String>) -> String {
    anchor.as_ref().map_or_else(String::new, |a| format!(" id=\"{}\"", esc(a)))
}

fn xhtml_page(language: &str, title: &str, stylesheet: &str, body: &str) -> String {
    let lang = esc(language);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" lang=\"{lang}\" xml:lang=\"{lang}\">\n\
         <head>\n<meta charset=\"UTF-8\"/>\n<title>{}</title>\n\
         <link rel=\"stylesheet\" type=\"text/css\" href=\"{stylesheet}\"/>\n</head>\n\
         <body>\n{body}</body>\n</html>\n",
        esc(title)
    )
}

fn chapter_xhtml(book: &Book, chapter: &Chapter, classes: &Classes) -> String {
    let mut body = String::new();
    for block in &chapter.blocks {
        match block {
            Block::Heading { level, anchor, style, spans } => {
                let level = (*level).clamp(1, 6);
                body.push_str(&format!(
                    "<h{level}{}{}>{}</h{level}>\n",
                    id_attr(anchor),
                    classes.class_attr(style),
                    spans_xhtml(spans)
                ));
            }
            Block::Paragraph { anchor, style, spans } => {
                body.push_str(&format!("<p{}{}>{}</p>\n", id_attr(anchor), classes.class_attr(style), spans_xhtml(spans)));
            }
            Block::Image { anchor, name, alt } => {
                body.push_str(&format!(
                    "<div class=\"img\"{}><img src=\"../images/{}\" alt=\"{}\"/></div>\n",
                    id_attr(anchor),
                    esc(name),
                    esc(alt)
                ));
            }
        }
    }
    xhtml_page(&book.meta.language, &book.meta.title, "../style.css", &body)
}

/// El índice del libro o, si no lo trae, una entrada por capítulo.
fn effective_toc(book: &Book, chapters: &[&Chapter]) -> Vec<TocEntry> {
    if !book.toc.is_empty() {
        return book.toc.clone();
    }
    chapters
        .iter()
        .enumerate()
        .map(|(i, chapter)| {
            let heading = chapter.blocks.iter().find_map(|b| match b {
                Block::Heading { spans, .. } => {
                    let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                    Some(text.trim().to_string()).filter(|t| !t.is_empty())
                }
                _ => None,
            });
            TocEntry {
                label: heading.unwrap_or_else(|| format!("Capítulo {}", i + 1)),
                target: chapter.id.clone(),
                children: Vec::new(),
            }
        })
        .collect()
}

fn nav_list(entries: &[TocEntry]) -> String {
    let mut out = String::from("<ol>\n");
    for entry in entries {
        out.push_str(&format!("<li><a href=\"{}\">{}</a>", esc(&href(&entry.target, "text/")), esc(&entry.label)));
        if !entry.children.is_empty() {
            out.push('\n');
            out.push_str(&nav_list(&entry.children));
        }
        out.push_str("</li>\n");
    }
    out.push_str("</ol>\n");
    out
}

fn nav_xhtml(book: &Book, toc: &[TocEntry]) -> String {
    let body = format!("<nav epub:type=\"toc\" id=\"toc\">\n<h1>Índice</h1>\n{}</nav>\n", nav_list(toc));
    xhtml_page(&book.meta.language, "Índice", "style.css", &body)
}

fn ncx_points(entries: &[TocEntry], counter: &mut usize) -> String {
    let mut out = String::new();
    for entry in entries {
        *counter += 1;
        out.push_str(&format!(
            "<navPoint id=\"n{0}\" playOrder=\"{0}\">\n<navLabel><text>{1}</text></navLabel>\n<content src=\"{2}\"/>\n",
            *counter,
            esc(&entry.label),
            esc(&href(&entry.target, "text/"))
        ));
        out.push_str(&ncx_points(&entry.children, counter));
        out.push_str("</navPoint>\n");
    }
    out
}

fn toc_depth(entries: &[TocEntry]) -> usize {
    entries.iter().map(|e| 1 + toc_depth(&e.children)).max().unwrap_or(0)
}

fn toc_ncx(book: &Book, toc: &[TocEntry], identifier: &str) -> String {
    let mut counter = 0;
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <ncx xmlns=\"http://www.daisy.org/z3986/2005/ncx/\" version=\"2005-1\">\n\
         <head>\n<meta name=\"dtb:uid\" content=\"{}\"/>\n<meta name=\"dtb:depth\" content=\"{}\"/>\n\
         <meta name=\"dtb:totalPageCount\" content=\"0\"/>\n<meta name=\"dtb:maxPageNumber\" content=\"0\"/>\n</head>\n\
         <docTitle><text>{}</text></docTitle>\n<navMap>\n{}</navMap>\n</ncx>\n",
        esc(identifier),
        toc_depth(toc).max(1),
        esc(&book.meta.title),
        ncx_points(toc, &mut counter)
    )
}

/// `urn:asin:<ASIN>`, o un UUID que depende solo del título y los autores.
fn identifier(book: &Book) -> String {
    if let Some(asin) = &book.meta.asin {
        return format!("urn:asin:{asin}");
    }
    let fnv = |seed: u64| {
        let mut hash = seed;
        for byte in book.meta.title.bytes().chain(book.meta.authors.iter().flat_map(|a| a.bytes())) {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01B3);
        }
        hash
    };
    let (a, b) = (fnv(0xCBF2_9CE4_8422_2325), fnv(0x8422_2325_CBF2_9CE4));
    format!(
        "urn:uuid:{:08x}-{:04x}-5{:03x}-{:04x}-{:012x}",
        (a >> 32) as u32,
        (a >> 16) as u16,
        a & 0x0FFF,
        ((b >> 48) as u16 & 0x3FFF) | 0x8000,
        b & 0xFFFF_FFFF_FFFF
    )
}

fn content_opf(book: &Book, chapters: &[&Chapter], identifier: &str, modified: &str) -> String {
    let mut metadata = format!(
        "<dc:identifier id=\"bookid\">{}</dc:identifier>\n<dc:title>{}</dc:title>\n<dc:language>{}</dc:language>\n",
        esc(identifier),
        esc(&book.meta.title),
        esc(&book.meta.language)
    );
    for author in &book.meta.authors {
        metadata.push_str(&format!("<dc:creator>{}</dc:creator>\n", esc(author)));
    }
    if let Some(publisher) = &book.meta.publisher {
        metadata.push_str(&format!("<dc:publisher>{}</dc:publisher>\n", esc(publisher)));
    }
    metadata.push_str(&format!("<meta property=\"dcterms:modified\">{modified}</meta>\n"));

    let mut manifest = String::from(
        "<item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n\
         <item id=\"ncx\" href=\"toc.ncx\" media-type=\"application/x-dtbncx+xml\"/>\n\
         <item id=\"css\" href=\"style.css\" media-type=\"text/css\"/>\n",
    );
    let mut spine = String::new();
    for (i, chapter) in chapters.iter().enumerate() {
        manifest.push_str(&format!(
            "<item id=\"ch{i}\" href=\"text/{}.xhtml\" media-type=\"application/xhtml+xml\"/>\n",
            esc(&chapter.id)
        ));
        spine.push_str(&format!("<itemref idref=\"ch{i}\"/>\n"));
    }
    for (i, image) in book.images.iter().enumerate() {
        let is_cover = book.cover.as_deref() == Some(image.name.as_str());
        if is_cover {
            metadata.push_str(&format!("<meta name=\"cover\" content=\"img{i}\"/>\n"));
        }
        manifest.push_str(&format!(
            "<item id=\"img{i}\" href=\"images/{}\" media-type=\"{}\"{}/>\n",
            esc(&image.name),
            esc(&image.mime),
            if is_cover { " properties=\"cover-image\"" } else { "" }
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"bookid\" xml:lang=\"{}\">\n\
         <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n{metadata}</metadata>\n\
         <manifest>\n{manifest}</manifest>\n<spine toc=\"ncx\">\n{spine}</spine>\n</package>\n",
        esc(&book.meta.language)
    )
}

const CONTAINER_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n\
<rootfiles>\n<rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/>\n</rootfiles>\n\
</container>\n";

fn zip_error(e: zip::result::ZipError) -> Error {
    Error::Other(format!("no se pudo escribir el EPUB: {e}"))
}

fn write_archive(book: &Book, chapters: &[&Chapter], file: std::fs::File) -> Result<(), Error> {
    let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let deflated = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut zip = zip::ZipWriter::new(file);
    let add = |zip: &mut zip::ZipWriter<std::fs::File>, name: &str, data: &[u8], compress: bool| {
        zip.start_file(name, if compress { deflated } else { stored }).map_err(zip_error)?;
        zip.write_all(data).map_err(Error::Io)
    };

    let classes = Classes::collect(chapters);
    let toc = effective_toc(book, chapters);
    let id = identifier(book);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());

    add(&mut zip, "mimetype", b"application/epub+zip", false)?;
    add(&mut zip, "META-INF/container.xml", CONTAINER_XML.as_bytes(), true)?;
    add(&mut zip, "OEBPS/content.opf", content_opf(book, chapters, &id, &format_utc(now)).as_bytes(), true)?;
    add(&mut zip, "OEBPS/nav.xhtml", nav_xhtml(book, &toc).as_bytes(), true)?;
    add(&mut zip, "OEBPS/toc.ncx", toc_ncx(book, &toc, &id).as_bytes(), true)?;
    add(&mut zip, "OEBPS/style.css", classes.css().as_bytes(), true)?;
    for chapter in chapters {
        let name = format!("OEBPS/text/{}.xhtml", chapter.id);
        add(&mut zip, &name, chapter_xhtml(book, chapter, &classes).as_bytes(), true)?;
    }
    for image in &book.images {
        // Las imágenes ya vienen comprimidas.
        add(&mut zip, &format!("OEBPS/images/{}", image.name), &image.data, false)?;
    }
    zip.finish().map_err(zip_error)?;
    Ok(())
}

/// Escribe el libro como EPUB 3 en `out`. Si algo falla, no deja un archivo a medias.
pub fn write_epub(book: &Book, out: &Path) -> Result<(), Error> {
    let chapters: Vec<&Chapter> = book.chapters.iter().filter(|c| !c.blocks.is_empty()).collect();
    if chapters.is_empty() {
        return Err(Error::Other("el libro no tiene contenido que convertir".to_string()));
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut partial = out.as_os_str().to_owned();
    partial.push(".part");
    let partial = std::path::PathBuf::from(partial);

    let result = std::fs::File::create(&partial)
        .map_err(Error::Io)
        .and_then(|file| write_archive(book, &chapters, file))
        .and_then(|()| std::fs::rename(&partial, out).map_err(Error::Io));
    if result.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::{Align, Block, Book, Chapter, Image, Length, Meta, Span, Style, TocEntry};
    use std::io::Read;

    fn span(text: &str) -> Span {
        Span { text: text.into(), bold: false, italic: false, link: None }
    }

    fn para(text: &str) -> Block {
        Block::Paragraph { anchor: None, style: Style::default(), spans: vec![span(text)] }
    }

    fn sample() -> Book {
        let centered = Style { align: Some(Align::Center), indent: Some(Length::Em(1.5)), ..Style::default() };
        Book {
            meta: Meta {
                title: "Mi libro & otros".into(),
                authors: vec!["Ana Pérez".into(), "Luis Gómez".into()],
                language: "es".into(),
                asin: Some("B0TEST".into()),
                publisher: Some("Editorial X".into()),
            },
            chapters: vec![
                Chapter {
                    id: "c0".into(),
                    blocks: vec![
                        Block::Image { anchor: None, name: "e1.jpg".into(), alt: "Mapa \"antiguo\"".into() },
                        Block::Heading {
                            level: 2,
                            anchor: Some("p10".into()),
                            style: centered.clone(),
                            spans: vec![span("Parte I")],
                        },
                        Block::Paragraph {
                            anchor: None,
                            style: centered.clone(),
                            spans: vec![
                                span("Tom & Jerry <3 "),
                                Span { text: "negrita".into(), bold: true, italic: false, link: None },
                                Span { text: "cursiva".into(), bold: false, italic: true, link: None },
                                Span { text: "ir".into(), bold: false, italic: false, link: Some("c1#p20".into()) },
                                Span { text: "web".into(), bold: false, italic: false, link: Some("https://example.org/a?b=1&c=2".into()) },
                            ],
                        },
                    ],
                },
                Chapter { id: "vacio".into(), blocks: vec![] },
                Chapter {
                    id: "c1".into(),
                    blocks: vec![
                        Block::Paragraph { anchor: Some("p20".into()), style: Style::default(), spans: vec![span("Capítulo 1")] },
                        para("a\u{1}b"),
                    ],
                },
            ],
            toc: vec![TocEntry {
                label: "Parte I".into(),
                target: "c0#p10".into(),
                children: vec![TocEntry { label: "Capítulo <1>".into(), target: "c1#p20".into(), children: vec![] }],
            }],
            images: vec![Image { name: "e1.jpg".into(), mime: "image/jpeg".into(), data: vec![0xFF, 0xD8, 0xFF, 0xE0] }],
            cover: Some("e1.jpg".into()),
            warnings: vec![],
        }
    }

    struct Epub {
        names: Vec<String>,
        files: std::collections::HashMap<String, Vec<u8>>,
        first_is_stored: bool,
    }

    impl Epub {
        fn text(&self, name: &str) -> String {
            String::from_utf8(self.files.get(name).unwrap_or_else(|| panic!("falta {name}: {:?}", self.names)).clone()).unwrap()
        }
    }

    fn written(book: &Book) -> Epub {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("out.epub");
        write_epub(book, &path).unwrap();
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
        let mut names = Vec::new();
        let mut files = std::collections::HashMap::new();
        let mut first_is_stored = false;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).unwrap();
            let name = entry.name().unwrap().into_owned();
            if i == 0 {
                first_is_stored = entry.compression() == zip::CompressionMethod::Stored;
            }
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            names.push(name.clone());
            files.insert(name, data);
        }
        Epub { names, files, first_is_stored }
    }

    /// Texto de todos los elementos con ese nombre.
    fn texts(xml: &str, tag: &str) -> Vec<String> {
        let doc = roxmltree::Document::parse(xml).unwrap();
        doc.descendants()
            .filter(|n| n.has_tag_name(tag))
            .map(|n| n.descendants().filter(|d| d.is_text()).filter_map(|d| d.text()).collect::<String>())
            .collect()
    }

    fn attrs(xml: &str, tag: &str, attr: &str) -> Vec<String> {
        let doc = roxmltree::Document::parse(xml).unwrap();
        doc.descendants().filter(|n| n.has_tag_name(tag)).filter_map(|n| n.attribute(attr).map(String::from)).collect()
    }

    #[test]
    fn mimetype_comes_first_and_uncompressed() {
        let epub = written(&sample());
        assert_eq!(epub.names[0], "mimetype");
        assert!(epub.first_is_stored);
        assert_eq!(epub.files["mimetype"], b"application/epub+zip");
    }

    #[test]
    fn has_the_expected_files() {
        let epub = written(&sample());
        for name in [
            "META-INF/container.xml",
            "OEBPS/content.opf",
            "OEBPS/nav.xhtml",
            "OEBPS/toc.ncx",
            "OEBPS/style.css",
            "OEBPS/text/c0.xhtml",
            "OEBPS/text/c1.xhtml",
            "OEBPS/images/e1.jpg",
        ] {
            assert!(epub.files.contains_key(name), "falta {name}: {:?}", epub.names);
        }
        assert_eq!(epub.files["OEBPS/images/e1.jpg"], vec![0xFF, 0xD8, 0xFF, 0xE0]);
        assert!(!epub.files.contains_key("OEBPS/text/vacio.xhtml"), "un capítulo vacío no genera archivo");
    }

    #[test]
    fn every_xml_document_is_well_formed() {
        let epub = written(&sample());
        let mut checked = 0;
        for name in &epub.names {
            if [".xhtml", ".opf", ".ncx", ".xml"].iter().any(|ext| name.ends_with(ext)) {
                let text = epub.text(name);
                roxmltree::Document::parse(&text).unwrap_or_else(|e| panic!("{name} mal formado: {e}\n{text}"));
                checked += 1;
            }
        }
        assert_eq!(checked, 6);
    }

    #[test]
    fn container_points_at_the_package() {
        let epub = written(&sample());
        assert_eq!(attrs(&epub.text("META-INF/container.xml"), "rootfile", "full-path"), vec!["OEBPS/content.opf"]);
    }

    #[test]
    fn text_is_escaped_and_survives_a_round_trip() {
        let epub = written(&sample());
        let chapter = epub.text("OEBPS/text/c0.xhtml");
        assert!(chapter.contains("Tom &amp; Jerry &lt;3 "), "{chapter}");
        let paragraphs = texts(&chapter, "p");
        assert_eq!(paragraphs, vec!["Tom & Jerry <3 negritacursivairweb"]);
        assert_eq!(attrs(&chapter, "img", "alt"), vec!["Mapa \"antiguo\""]);
    }

    #[test]
    fn control_characters_are_dropped() {
        let epub = written(&sample());
        assert_eq!(texts(&epub.text("OEBPS/text/c1.xhtml"), "p"), vec!["Capítulo 1", "ab"]);
    }

    #[test]
    fn inline_formatting_becomes_elements() {
        let epub = written(&sample());
        let chapter = epub.text("OEBPS/text/c0.xhtml");
        assert_eq!(texts(&chapter, "b"), vec!["negrita"]);
        assert_eq!(texts(&chapter, "i"), vec!["cursiva"]);
    }

    #[test]
    fn links_point_at_chapter_files_and_urls_stay_as_they_are() {
        let epub = written(&sample());
        let chapter = epub.text("OEBPS/text/c0.xhtml");
        assert_eq!(attrs(&chapter, "a", "href"), vec!["c1.xhtml#p20", "https://example.org/a?b=1&c=2"]);
        assert_eq!(texts(&chapter, "a"), vec!["ir", "web"]);
    }

    #[test]
    fn anchors_headings_and_images_are_written() {
        let epub = written(&sample());
        let c0 = epub.text("OEBPS/text/c0.xhtml");
        assert_eq!(texts(&c0, "h2"), vec!["Parte I"]);
        assert_eq!(attrs(&c0, "h2", "id"), vec!["p10"]);
        assert_eq!(attrs(&c0, "img", "src"), vec!["../images/e1.jpg"]);
        assert_eq!(attrs(&epub.text("OEBPS/text/c1.xhtml"), "p", "id"), vec!["p20"]);
        assert_eq!(attrs(&c0, "html", "lang"), vec!["es"]);
    }

    #[test]
    fn equal_styles_share_one_css_class() {
        let epub = written(&sample());
        let c0 = epub.text("OEBPS/text/c0.xhtml");
        let heading_class = attrs(&c0, "h2", "class");
        let paragraph_class = attrs(&c0, "p", "class");
        assert_eq!(heading_class.len(), 1);
        assert_eq!(heading_class, paragraph_class);

        let css = epub.text("OEBPS/style.css");
        let rule = css
            .split('}')
            .find(|r| r.contains(&format!(".{}", heading_class[0])))
            .unwrap_or_else(|| panic!("la clase {} no está en el CSS:\n{css}", heading_class[0]));
        assert!(rule.contains("text-align: center"), "{rule}");
        assert!(rule.contains("text-indent: 1.5em"), "{rule}");
        assert!(attrs(&epub.text("OEBPS/text/c1.xhtml"), "p", "class").is_empty(), "sin estilo, sin clase");
    }

    #[test]
    fn css_covers_every_length_unit_and_flag() {
        let mut book = sample();
        book.chapters[2].blocks.push(Block::Paragraph {
            anchor: None,
            style: Style {
                align: Some(Align::Justify),
                indent: Some(Length::Percent(5.0)),
                margin_top: Some(Length::Em(2.4)),
                margin_bottom: Some(Length::Pt(12.0)),
                font_size: Some(Length::Rem(1.6)),
                bold: true,
                italic: true,
            },
            spans: vec![span("x")],
        });
        let css = written(&book).text("OEBPS/style.css");
        for expected in [
            "text-align: justify",
            "text-indent: 5%",
            "margin-top: 2.4em",
            "margin-bottom: 12pt",
            "font-size: 1.6rem",
            "font-weight: bold",
            "font-style: italic",
        ] {
            assert!(css.contains(expected), "falta `{expected}` en:\n{css}");
        }
    }

    #[test]
    fn package_lists_metadata_manifest_and_spine() {
        let epub = written(&sample());
        let opf = epub.text("OEBPS/content.opf");
        assert_eq!(texts(&opf, "title"), vec!["Mi libro & otros"]);
        assert_eq!(texts(&opf, "creator"), vec!["Ana Pérez", "Luis Gómez"]);
        assert_eq!(texts(&opf, "language"), vec!["es"]);
        assert_eq!(texts(&opf, "identifier"), vec!["urn:asin:B0TEST"]);
        assert_eq!(texts(&opf, "publisher"), vec!["Editorial X"]);
        assert_eq!(attrs(&opf, "package", "version"), vec!["3.0"]);

        let doc = roxmltree::Document::parse(&opf).unwrap();
        let item = |href: &str| {
            doc.descendants()
                .find(|n| n.has_tag_name("item") && n.attribute("href") == Some(href))
                .unwrap_or_else(|| panic!("falta {href} en el manifiesto"))
        };
        assert_eq!(item("nav.xhtml").attribute("properties"), Some("nav"));
        assert_eq!(item("images/e1.jpg").attribute("properties"), Some("cover-image"));
        assert_eq!(item("images/e1.jpg").attribute("media-type"), Some("image/jpeg"));
        assert_eq!(item("text/c0.xhtml").attribute("media-type"), Some("application/xhtml+xml"));
        assert_eq!(item("toc.ncx").attribute("media-type"), Some("application/x-dtbncx+xml"));
        assert_eq!(item("style.css").attribute("media-type"), Some("text/css"));

        let spine: Vec<&str> = doc
            .descendants()
            .filter(|n| n.has_tag_name("itemref"))
            .filter_map(|n| n.attribute("idref"))
            .map(|idref| {
                doc.descendants().find(|n| n.has_tag_name("item") && n.attribute("id") == Some(idref)).unwrap().attribute("href").unwrap()
            })
            .collect();
        assert_eq!(spine, vec!["text/c0.xhtml", "text/c1.xhtml"]);

        let modified = doc
            .descendants()
            .find(|n| n.has_tag_name("meta") && n.attribute("property") == Some("dcterms:modified"))
            .and_then(|n| n.text())
            .unwrap();
        assert_eq!(modified.len(), 20, "{modified}");
        assert!(modified.ends_with('Z'), "{modified}");
    }

    #[test]
    fn book_without_asin_gets_a_stable_uuid() {
        let mut book = sample();
        book.meta.asin = None;
        let id = |b: &Book| texts(&written(b).text("OEBPS/content.opf"), "identifier").remove(0);
        let first = id(&book);
        assert!(first.starts_with("urn:uuid:"), "{first}");
        assert_eq!(first.len(), "urn:uuid:".len() + 36);
        assert_eq!(first, id(&book), "el identificador no debe cambiar entre conversiones");
    }

    #[test]
    fn navigation_nests_and_links_into_chapters() {
        let epub = written(&sample());
        let nav = epub.text("OEBPS/nav.xhtml");
        assert_eq!(attrs(&nav, "a", "href"), vec!["text/c0.xhtml#p10", "text/c1.xhtml#p20"]);
        assert_eq!(texts(&nav, "a"), vec!["Parte I", "Capítulo <1>"]);
        let doc = roxmltree::Document::parse(&nav).unwrap();
        let lists = doc.descendants().filter(|n| n.has_tag_name("ol")).count();
        assert_eq!(lists, 2, "la entrada hija va en una lista anidada");

        let ncx = epub.text("OEBPS/toc.ncx");
        assert_eq!(attrs(&ncx, "content", "src"), vec!["text/c0.xhtml#p10", "text/c1.xhtml#p20"]);
        assert_eq!(attrs(&ncx, "navPoint", "playOrder"), vec!["1", "2"]);
    }

    #[test]
    fn book_without_toc_lists_its_chapters() {
        let mut book = sample();
        book.toc.clear();
        let nav = written(&book).text("OEBPS/nav.xhtml");
        assert_eq!(attrs(&nav, "a", "href"), vec!["text/c0.xhtml", "text/c1.xhtml"]);
        assert_eq!(texts(&nav, "a"), vec!["Parte I", "Capítulo 2"], "usa el primer encabezado o un número");
    }

    #[test]
    fn book_without_content_is_an_error_and_writes_nothing() {
        let mut book = sample();
        for chapter in &mut book.chapters {
            chapter.blocks.clear();
        }
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("out.epub");
        assert!(write_epub(&book, &path).is_err());
        assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 0);
    }

    #[test]
    fn creates_missing_parent_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("a").join("b").join("out.epub");
        write_epub(&sample(), &path).unwrap();
        assert!(path.is_file());
    }

    #[test]
    fn file_names_are_safe_on_windows() {
        assert_eq!(safe_file_name("New Life: An OP MC?"), "New Life_ An OP MC_");
        assert_eq!(safe_file_name("a<b>c\"d/e\\f|g*h"), "a_b_c_d_e_f_g_h");
        assert_eq!(safe_file_name("Señor de los años"), "Señor de los años");
        assert_eq!(safe_file_name("  Hola. . "), "Hola");
        assert_eq!(safe_file_name("tab\there"), "tab_here");
    }

    #[test]
    fn empty_and_reserved_names_get_a_placeholder() {
        for name in ["", "   ", "...", "CON", "con", "NUL", "nul.txt", "COM1", "lpt9", "AUX"] {
            assert_eq!(safe_file_name(name), "libro", "{name:?}");
        }
        assert_eq!(safe_file_name("CONSOLA"), "CONSOLA");
    }

    #[test]
    fn long_names_are_cut_to_120_characters() {
        let name = safe_file_name(&"ñ".repeat(300));
        assert_eq!(name.chars().count(), 120);
        let cut_on_a_space = safe_file_name(&format!("{}{}", "a".repeat(119), " b"));
        assert_eq!(cut_on_a_space, "a".repeat(119), "no debe terminar en espacio");
    }

    #[test]
    fn utc_timestamps_are_formatted_without_a_date_library() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(format_utc(1_700_000_000), "2023-11-14T22:13:20Z");
        assert_eq!(format_utc(4_102_444_799), "2099-12-31T23:59:59Z");
    }

    mod convert {
        use crate::testkit::*;
        use crate::yj::*;

        fn tiny_kfx(title: Option<&str>) -> Vec<u8> {
            let (section, story, content) = (FIRST_LOCAL, FIRST_LOCAL + 1, FIRST_LOCAL + 2);
            let order = st(vec![(178, sym(351)), (SECTIONS, list(vec![sym(section)]))]);
            let template = st(vec![(POSITION_ID, int(1)), (STORY_NAME, sym(story)), (NODE_TYPE, sym(NODE_TEXT))]);
            let node = st(vec![
                (POSITION_ID, int(10)),
                (NODE_TYPE, sym(NODE_TEXT)),
                (CONTENT, st(vec![(NAME, sym(content)), (INDEX, int(0))])),
            ]);
            let mut entities = vec![
                TestEntity { id: 348, ftype: 538, payload: ion_doc(&st(vec![(READING_ORDERS, list(vec![order]))])) },
                TestEntity {
                    id: section,
                    ftype: 260,
                    payload: ion_doc(&st(vec![(SECTION_NAME, sym(section)), (PAGE_TEMPLATES, list(vec![template]))])),
                },
                TestEntity {
                    id: story,
                    ftype: 259,
                    payload: ion_doc(&st(vec![(STORY_NAME, sym(story)), (CONTENT_LIST, list(vec![node]))])),
                },
                TestEntity {
                    id: content,
                    ftype: 145,
                    payload: ion_doc(&st(vec![(NAME, sym(content)), (CONTENT_LIST, list(vec![s("Hola mundo")]))])),
                },
            ];
            if let Some(title) = title {
                let entry = st(vec![(META_KEY, s("title")), (META_VALUE, s(title))]);
                let category = st(vec![(META_CATEGORY, s("kindle_title_metadata")), (META_ENTRIES, list(vec![entry]))]);
                entities.push(TestEntity {
                    id: 348,
                    ftype: 490,
                    payload: ion_doc(&st(vec![(META_CATEGORIES, list(vec![category]))])),
                });
            }
            build_container(&["c0", "l0", "k"], &entities)
        }

        #[test]
        fn converts_a_kfx_file_into_an_epub_named_after_its_title() {
            let tmp = tempfile::tempdir().unwrap();
            let input = tmp.path().join("in.kfx");
            std::fs::write(&input, tiny_kfx(Some("Un libro: prueba?"))).unwrap();
            let out_dir = tmp.path().join("salida");

            let converted = crate::convert(&input, &out_dir).unwrap();

            assert_eq!(converted.title, "Un libro: prueba?");
            assert_eq!(converted.path, out_dir.join("Un libro_ prueba_.epub"));
            assert!(converted.path.is_file());
            assert!(converted.warnings.is_empty(), "{:?}", converted.warnings);
            let mut archive = zip::ZipArchive::new(std::fs::File::open(&converted.path).unwrap()).unwrap();
            let mut chapter = String::new();
            std::io::Read::read_to_string(&mut archive.by_name("OEBPS/text/c0.xhtml").unwrap(), &mut chapter).unwrap();
            assert!(chapter.contains("Hola mundo"), "{chapter}");
        }

        #[test]
        fn never_overwrites_an_existing_epub() {
            let tmp = tempfile::tempdir().unwrap();
            let input = tmp.path().join("in.kfx");
            std::fs::write(&input, tiny_kfx(Some("Libro"))).unwrap();

            let first = crate::convert(&input, tmp.path()).unwrap();
            let second = crate::convert(&input, tmp.path()).unwrap();
            let third = crate::convert(&input, tmp.path()).unwrap();

            assert_eq!(first.path, tmp.path().join("Libro.epub"));
            assert_eq!(second.path, tmp.path().join("Libro (2).epub"));
            assert_eq!(third.path, tmp.path().join("Libro (3).epub"));
            assert!(first.path.is_file() && second.path.is_file() && third.path.is_file());
        }

        #[test]
        fn title_is_read_without_converting() {
            let tmp = tempfile::tempdir().unwrap();
            let input = tmp.path().join("in.kfx");
            std::fs::write(&input, tiny_kfx(Some("Un libro"))).unwrap();
            assert_eq!(crate::read_title(&input).as_deref(), Some("Un libro"));
            std::fs::write(&input, tiny_kfx(None)).unwrap();
            assert_eq!(crate::read_title(&input), None);
            assert_eq!(crate::read_title(&tmp.path().join("no-existe.kfx")), None);
        }

        #[test]
        fn untitled_book_still_converts() {
            let tmp = tempfile::tempdir().unwrap();
            let input = tmp.path().join("in.kfx");
            std::fs::write(&input, tiny_kfx(None)).unwrap();
            let converted = crate::convert(&input, tmp.path()).unwrap();
            assert_eq!(converted.path, tmp.path().join("Sin título.epub"));
        }

        #[test]
        fn failed_conversion_leaves_no_file_behind() {
            let tmp = tempfile::tempdir().unwrap();
            let input = tmp.path().join("in.kfx");
            std::fs::write(&input, b"\xeaDRMION\xee cifrado").unwrap();
            let out_dir = tmp.path().join("salida");

            assert!(matches!(crate::convert(&input, &out_dir), Err(crate::Error::StillEncrypted)));
            let leftovers = std::fs::read_dir(&out_dir).map(|d| d.count()).unwrap_or(0);
            assert_eq!(leftovers, 0);
        }
    }
}
