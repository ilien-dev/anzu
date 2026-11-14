//! Modelo intermedio de un libro: lo que el conversor entiende de un KFX.
//!
//! Recorrido: orden de lectura -> secciones -> storylines -> nodos de contenido. El texto
//! de cada nodo se resuelve por referencia a un fragmento de contenido; los enlaces y la
//! tabla de contenidos apuntan a posiciones, que aquí se traducen a anclas de capítulo.
//!
//! Lo que no se reconoce se omite y deja un aviso en `Book::warnings`; solo la falta de
//! orden de lectura impide construir el libro.

use std::collections::{HashMap, HashSet};

use crate::Error;
use crate::container::{FragmentValue, Fragments};
use crate::ion::Ion;
use crate::yj::*;

/// Una "altura de línea" de KFX expresada en em.
const LH_IN_EM: f64 = 1.2;

#[derive(Debug, Clone, PartialEq)]
pub struct Book {
    pub meta: Meta,
    pub chapters: Vec<Chapter>,
    pub toc: Vec<TocEntry>,
    pub images: Vec<Image>,
    /// Nombre de archivo de la imagen de portada, si la hay.
    pub cover: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    pub title: String,
    pub authors: Vec<String>,
    pub language: String,
    pub asin: Option<String>,
    pub publisher: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chapter {
    pub id: String,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading { level: u8, anchor: Option<String>, style: Style, spans: Vec<Span> },
    Paragraph { anchor: Option<String>, style: Style, spans: Vec<Span> },
    Image { anchor: Option<String>, name: String, alt: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    /// `capítulo#ancla` para un enlace interno, o una URL.
    pub link: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
    Center,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    Em(f64),
    Percent(f64),
    Rem(f64),
    Pt(f64),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Style {
    pub align: Option<Align>,
    pub indent: Option<Length>,
    pub margin_top: Option<Length>,
    pub margin_bottom: Option<Length>,
    pub font_size: Option<Length>,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocEntry {
    pub label: String,
    /// `capítulo#ancla`.
    pub target: String,
    pub children: Vec<TocEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub name: String,
    pub mime: String,
    pub data: Vec<u8>,
}

/// Construye el modelo del libro a partir de sus fragmentos.
pub fn build(fragments: &Fragments) -> Result<Book, Error> {
    let mut builder = Builder::new(fragments);
    let order = builder.reading_order()?;
    for section in order {
        builder.section(section);
    }
    let chapter_ids: Vec<String> = builder.chapters.iter().map(|c| c.id.clone()).collect();
    builder.resolve_links(&chapter_ids);
    let toc = builder.toc(&chapter_ids);
    let (meta, cover_resource) = builder.metadata();
    let cover = cover_resource.and_then(|r| builder.image_file(&r));
    builder.place_anchors();
    Ok(Book { meta, chapters: builder.chapters, toc, images: builder.images, cover, warnings: builder.warnings })
}

/// Solo el título del libro, sin construir el resto del modelo.
pub fn title(fragments: &Fragments) -> Option<String> {
    for fragment in fragments.of_type(F_BOOK_METADATA) {
        let FragmentValue::Ion(value) = &fragment.value else { continue };
        for category in value.field(META_CATEGORIES).and_then(items).unwrap_or(&[]) {
            for entry in category.field(META_ENTRIES).and_then(items).unwrap_or(&[]) {
                if entry.field(META_KEY).and_then(string) == Some("title")
                    && let Some(text) = entry.field(META_VALUE).and_then(string).map(str::trim).filter(|t| !t.is_empty())
                {
                    return Some(text.to_string());
                }
            }
        }
    }
    None
}

fn as_int(v: &Ion) -> Option<i64> {
    match v.unannotated() {
        Ion::Int(i) => Some(*i),
        _ => None,
    }
}

fn string(v: &Ion) -> Option<&str> {
    match v.unannotated() {
        Ion::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn items(v: &Ion) -> Option<&[Ion]> {
    match v.unannotated() {
        Ion::List(items) | Ion::Sexp(items) => Some(items.as_slice()),
        _ => None,
    }
}

/// Un nombre apto para archivo e identificador XML.
fn sanitize(name: &str) -> String {
    let clean: String =
        name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    if clean.is_empty() { "x".to_string() } else { clean }
}

enum AnchorTarget<'a> {
    Position(i64),
    Url(&'a str),
}

/// Lo que un evento de estilo aporta a un tramo de texto.
struct Event<'a> {
    start: usize,
    end: usize,
    bold: bool,
    italic: bool,
    link: Option<&'a str>,
}

struct Builder<'a> {
    f: &'a Fragments,
    ion: HashMap<(&'a str, &'a str), &'a Ion>,
    raw: HashMap<&'a str, &'a [u8]>,
    warnings: Vec<String>,
    chapters: Vec<Chapter>,
    /// Posición de cada bloque, en paralelo a `chapters[i].blocks`.
    block_positions: Vec<Vec<Option<i64>>>,
    /// Posición -> (capítulo, posición del bloque que la representa).
    targets: HashMap<i64, (usize, i64)>,
    /// Posiciones de contenedores que esperan a su primer bloque.
    pending: Vec<i64>,
    images: Vec<Image>,
    image_names: HashMap<String, Option<String>>,
    referenced: HashSet<i64>,
    used_ids: HashSet<String>,
}

impl<'a> Builder<'a> {
    fn new(f: &'a Fragments) -> Self {
        let mut ion = HashMap::new();
        let mut raw = HashMap::new();
        for fragment in &f.items {
            match &fragment.value {
                FragmentValue::Ion(v) => {
                    ion.entry((fragment.ftype.as_str(), fragment.fid.as_str())).or_insert(v);
                }
                FragmentValue::Raw(bytes) if fragment.ftype == F_RAW_MEDIA => {
                    raw.entry(fragment.fid.as_str()).or_insert(bytes.as_slice());
                }
                FragmentValue::Raw(_) => {}
            }
        }
        Builder {
            f,
            ion,
            raw,
            warnings: Vec::new(),
            chapters: Vec::new(),
            block_positions: Vec::new(),
            targets: HashMap::new(),
            pending: Vec::new(),
            images: Vec::new(),
            image_names: HashMap::new(),
            referenced: HashSet::new(),
            used_ids: HashSet::new(),
        }
    }

    fn warn(&mut self, message: String) {
        if !self.warnings.contains(&message) {
            self.warnings.push(message);
        }
    }

    fn get(&self, ftype: &'a str, fid: &str) -> Option<&'a Ion> {
        // La clave pide `&'a str` para ambos; `fid` solo se usa para comparar.
        self.ion.iter().find(|((t, id), _)| *t == ftype && *id == fid).map(|(_, v)| *v)
    }

    fn symbol_name(&self, sid: u32) -> String {
        self.f.symbols.name(sid).map_or_else(|| format!("${sid}"), str::to_string)
    }

    /// El nombre al que se refiere un símbolo (o una cadena usada como referencia).
    fn name_of(&self, v: &'a Ion) -> Option<&'a str> {
        match v.unannotated() {
            Ion::Symbol(sid) => self.f.symbols.name(*sid),
            Ion::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn reading_order(&self) -> Result<Vec<&'a str>, Error> {
        for ftype in [F_DOCUMENT, F_METADATA_OLD] {
            for fragment in self.f.of_type(ftype) {
                let FragmentValue::Ion(value) = &fragment.value else { continue };
                let Some(orders) = value.field(READING_ORDERS).and_then(items) else { continue };
                for order in orders {
                    let Some(sections) = order.field(SECTIONS).and_then(items) else { continue };
                    let names: Vec<&'a str> = sections.iter().filter_map(|s| self.name_of(s)).collect();
                    if !names.is_empty() {
                        return Ok(names);
                    }
                }
            }
        }
        Err(Error::NoReadingOrder)
    }

    fn unique_id(&mut self, name: &str) -> String {
        let base = sanitize(name);
        let mut id = base.clone();
        let mut n = 2;
        while !self.used_ids.insert(id.clone()) {
            id = format!("{base}_{n}");
            n += 1;
        }
        id
    }

    fn section(&mut self, name: &'a str) {
        let Some(section) = self.get(F_SECTION, name) else {
            self.warn(format!("sección {name} no encontrada"));
            return;
        };
        let chapter = self.chapters.len();
        let id = self.unique_id(name);
        self.chapters.push(Chapter { id, blocks: Vec::new() });
        self.block_positions.push(Vec::new());

        for template in section.field(PAGE_TEMPLATES).and_then(items).unwrap_or(&[]) {
            match template.field(STORY_NAME).and_then(|s| self.name_of(s)) {
                Some(story) => match self.get(F_STORYLINE, story) {
                    Some(storyline) => {
                        for node in storyline.field(CONTENT_LIST).and_then(items).unwrap_or(&[]) {
                            self.node(node, chapter);
                        }
                    }
                    None => self.warn(format!("contenido {story} de la sección {name} no encontrado")),
                },
                None => self.node(template, chapter),
            }
        }
        self.pending.clear();
    }

    fn node(&mut self, node: &'a Ion, chapter: usize) {
        if !matches!(node.unannotated(), Ion::Struct(_)) {
            return;
        }
        let position = node.field(POSITION_ID).and_then(as_int);
        let kind = match node.field(NODE_TYPE).map(Ion::unannotated) {
            Some(Ion::Symbol(sid)) => Some(*sid),
            _ => None,
        };
        if kind == Some(NODE_IMAGE) {
            self.image_node(node, chapter, position);
            return;
        }
        if let Some(other) = kind.filter(|k| *k != NODE_TEXT && *k != NODE_CONTAINER) {
            // Listas, tablas y demás: se pierde la estructura, pero no el texto que contienen.
            let name = self.symbol_name(other);
            self.warn(format!("tipo de nodo desconocido {name}; se conserva solo su texto"));
        }
        let emitted = node.field(CONTENT).is_some_and(|r| self.text_node(node, r, chapter, position));
        if !emitted && let Some(p) = position {
            self.pending.push(p);
        }
        for child in node.field(CONTENT_LIST).and_then(items).unwrap_or(&[]) {
            self.node(child, chapter);
        }
    }

    fn emit(&mut self, chapter: usize, position: Option<i64>, block: Block) {
        self.chapters[chapter].blocks.push(block);
        self.block_positions[chapter].push(position);
        if let Some(p) = position {
            self.targets.entry(p).or_insert((chapter, p));
            for waiting in self.pending.drain(..) {
                self.targets.entry(waiting).or_insert((chapter, p));
            }
        }
    }

    /// Devuelve `true` si el nodo produjo un bloque.
    fn text_node(&mut self, node: &'a Ion, reference: &'a Ion, chapter: usize, position: Option<i64>) -> bool {
        let text = match reference.unannotated() {
            Ion::String(s) => Some(s.as_str()),
            _ => {
                let name = reference.field(NAME).and_then(|v| self.name_of(v));
                let index = reference.field(INDEX).and_then(as_int);
                let found = name.zip(index).and_then(|(name, index)| {
                    let list = self.get(F_CONTENT, name)?.field(CONTENT_LIST).and_then(items)?;
                    string(list.get(usize::try_from(index).ok()?)?)
                });
                if found.is_none() {
                    self.warn(format!(
                        "texto no encontrado: {}[{}]",
                        name.unwrap_or("?"),
                        index.map_or_else(|| "?".to_string(), |i| i.to_string())
                    ));
                }
                found
            }
        };
        let Some(text) = text.filter(|t| !t.is_empty()) else { return false };

        let events = self.events(node, text.chars().count());
        let spans = split_spans(text, &events);
        let style = node.field(STYLE).and_then(|s| self.name_of(s)).map(|n| self.style(n)).unwrap_or_default();
        let block = match node.field(HEADING_LEVEL).and_then(as_int) {
            Some(level) => Block::Heading { level: level.clamp(1, 6) as u8, anchor: None, style, spans },
            None => Block::Paragraph { anchor: None, style, spans },
        };
        self.emit(chapter, position, block);
        true
    }

    fn events(&self, node: &'a Ion, len: usize) -> Vec<Event<'a>> {
        let mut events = Vec::new();
        for event in node.field(STYLE_EVENTS).and_then(items).unwrap_or(&[]) {
            let (Some(offset), Some(length)) =
                (event.field(OFFSET).and_then(as_int), event.field(LENGTH).and_then(as_int))
            else {
                continue;
            };
            let Ok(start) = usize::try_from(offset) else { continue };
            if start >= len || length <= 0 {
                continue;
            }
            let end = start.saturating_add(usize::try_from(length).unwrap_or(usize::MAX)).min(len);
            let style = event.field(STYLE).and_then(|s| self.name_of(s)).map(|n| self.style(n)).unwrap_or_default();
            let link = event.field(LINK_TO).and_then(|a| self.name_of(a));
            events.push(Event { start, end, bold: style.bold, italic: style.italic, link });
        }
        events
    }

    fn style(&self, name: &str) -> Style {
        let Some(props) = self.get(F_STYLE, name) else { return Style::default() };
        let symbol = |sid: u32| match props.field(sid).map(Ion::unannotated) {
            Some(Ion::Symbol(s)) => Some(*s),
            _ => None,
        };
        let length = |sid: u32| {
            let measure = props.field(sid)?;
            let value = measure.field(VALUE)?.as_f64()?;
            match measure.field(UNIT).map(Ion::unannotated)? {
                Ion::Symbol(UNIT_EM) => Some(Length::Em(value)),
                Ion::Symbol(UNIT_LH) => Some(Length::Em(value * LH_IN_EM)),
                Ion::Symbol(UNIT_PERCENT) => Some(Length::Percent(value)),
                Ion::Symbol(UNIT_REM) => Some(Length::Rem(value)),
                Ion::Symbol(UNIT_PT) => Some(Length::Pt(value)),
                _ => None,
            }
        };
        Style {
            align: match symbol(TEXT_ALIGN) {
                Some(ALIGN_LEFT) => Some(Align::Left),
                Some(ALIGN_RIGHT) => Some(Align::Right),
                Some(ALIGN_CENTER) => Some(Align::Center),
                Some(ALIGN_JUSTIFY) => Some(Align::Justify),
                _ => None,
            },
            indent: length(TEXT_INDENT),
            margin_top: length(MARGIN_TOP),
            margin_bottom: length(MARGIN_BOTTOM),
            font_size: length(FONT_SIZE).filter(|l| *l != Length::Rem(1.0)),
            bold: symbol(FONT_WEIGHT).is_some_and(|w| (WEIGHT_BOLD_FROM..=WEIGHT_BOLD_TO).contains(&w)),
            italic: symbol(FONT_STYLE) == Some(ITALIC),
        }
    }

    fn image_node(&mut self, node: &'a Ion, chapter: usize, position: Option<i64>) {
        let Some(resource) = node.field(RESOURCE_NAME).and_then(|v| self.name_of(v)) else {
            self.warn("imagen sin recurso; se omite".to_string());
            return;
        };
        let Some(name) = self.image_file(resource) else { return };
        let alt = node.field(ALT_TEXT).and_then(string).unwrap_or("").to_string();
        self.emit(chapter, position, Block::Image { anchor: None, name, alt });
    }

    /// Nombre de archivo de la imagen de un recurso, añadiéndola al libro la primera vez.
    fn image_file(&mut self, resource: &str) -> Option<String> {
        if let Some(known) = self.image_names.get(resource) {
            return known.clone();
        }
        let result = self.load_image(resource);
        self.image_names.insert(resource.to_string(), result.clone());
        result
    }

    fn load_image(&mut self, resource: &str) -> Option<String> {
        let Some(info) = self.get(F_RESOURCE, resource) else {
            self.warn(format!("recurso {resource} no encontrado; se omite la imagen"));
            return None;
        };
        let data = info.field(LOCATION).and_then(string).and_then(|l| self.raw.get(l).copied());
        let Some(data) = data else {
            self.warn(format!("la imagen {resource} no tiene datos; se omite"));
            return None;
        };
        let declared = info.field(MIME).and_then(string).unwrap_or("");
        let (mime, extension) = match declared {
            "image/jpg" | "image/jpeg" => ("image/jpeg".to_string(), "jpg".to_string()),
            "image/png" => ("image/png".to_string(), "png".to_string()),
            "image/gif" => ("image/gif".to_string(), "gif".to_string()),
            "image/svg+xml" => ("image/svg+xml".to_string(), "svg".to_string()),
            other => {
                self.warn(format!("la imagen {resource} usa un formato poco común ({other})"));
                let subtype = other.rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or("bin");
                (if other.is_empty() { "application/octet-stream".to_string() } else { other.to_string() }, sanitize(subtype))
            }
        };
        let name = format!("{}.{extension}", sanitize(resource));
        self.images.push(Image { name: name.clone(), mime, data: data.to_vec() });
        Some(name)
    }

    fn anchors(&self) -> HashMap<&'a str, AnchorTarget<'a>> {
        let mut anchors = HashMap::new();
        for fragment in self.f.of_type(F_ANCHOR) {
            let FragmentValue::Ion(value) = &fragment.value else { continue };
            if let Some(url) = value.field(ANCHOR_URI).and_then(string) {
                anchors.insert(fragment.fid.as_str(), AnchorTarget::Url(url));
            } else if let Some(p) = value.field(POSITION).and_then(|p| p.field(POSITION_ID)).and_then(as_int) {
                anchors.insert(fragment.fid.as_str(), AnchorTarget::Position(p));
            }
        }
        anchors
    }

    /// Una posición convertida en `capítulo#ancla`, marcando el bloque como destino.
    fn target(&mut self, position: i64, chapter_ids: &[String]) -> Option<String> {
        let (chapter, block) = *self.targets.get(&position)?;
        self.referenced.insert(block);
        Some(format!("{}#p{block}", chapter_ids[chapter]))
    }

    /// Durante el recorrido, `Span::link` guarda el nombre del ancla; aquí se resuelve.
    fn resolve_links(&mut self, chapter_ids: &[String]) {
        let anchors = self.anchors();
        let mut chapters = std::mem::take(&mut self.chapters);
        for chapter in &mut chapters {
            for block in &mut chapter.blocks {
                let (Block::Heading { spans, .. } | Block::Paragraph { spans, .. }) = block else { continue };
                for span in spans.iter_mut() {
                    let Some(anchor) = span.link.take() else { continue };
                    span.link = match anchors.get(anchor.as_str()) {
                        Some(AnchorTarget::Url(url)) => Some(url.to_string()),
                        Some(AnchorTarget::Position(p)) => self.target(*p, chapter_ids),
                        None => None,
                    };
                    if span.link.is_none() {
                        self.warn(format!("enlace a un destino desconocido ({anchor}); se deja como texto"));
                    }
                }
                merge_equal_spans(spans);
            }
        }
        self.chapters = chapters;
    }

    fn toc(&mut self, chapter_ids: &[String]) -> Vec<TocEntry> {
        let Some(navigation) = self.f.of_type(F_NAVIGATION).find_map(|f| match &f.value {
            FragmentValue::Ion(v) => Some(v),
            FragmentValue::Raw(_) => None,
        }) else {
            return Vec::new();
        };
        let roots: &[Ion] = items(navigation).unwrap_or(std::slice::from_ref(navigation));
        for root in roots {
            for container in root.field(NAV_CONTAINERS).and_then(items).unwrap_or(&[]) {
                let is_toc = matches!(container.field(NAV_TYPE).map(Ion::unannotated), Some(Ion::Symbol(NAV_TOC)));
                if is_toc {
                    let entries = container.field(NAV_ENTRIES).and_then(items).unwrap_or(&[]);
                    return self.toc_entries(entries, chapter_ids);
                }
            }
        }
        Vec::new()
    }

    fn toc_entries(&mut self, entries: &'a [Ion], chapter_ids: &[String]) -> Vec<TocEntry> {
        let mut out = Vec::new();
        for entry in entries {
            let label = entry.field(NAV_REPRESENTATION).and_then(|r| r.field(NAV_LABEL)).and_then(string);
            let position = entry.field(NAV_TARGET).and_then(|t| t.field(POSITION_ID)).and_then(as_int);
            let children = self.toc_entries(entry.field(NAV_ENTRIES).and_then(items).unwrap_or(&[]), chapter_ids);
            let target = position.and_then(|p| self.target(p, chapter_ids));
            match (label.map(str::trim).filter(|l| !l.is_empty()), target) {
                (Some(label), Some(target)) => out.push(TocEntry { label: label.to_string(), target, children }),
                (label, _) => {
                    self.warn(format!(
                        "entrada del índice sin destino válido ({}); se omite",
                        label.unwrap_or("sin título")
                    ));
                    out.extend(children);
                }
            }
        }
        out
    }

    fn metadata(&self) -> (Meta, Option<String>) {
        let mut title = None;
        let mut authors = Vec::new();
        let mut language = None;
        let mut asin = None;
        let mut publisher = None;
        let mut cover = None;
        for fragment in self.f.of_type(F_BOOK_METADATA) {
            let FragmentValue::Ion(value) = &fragment.value else { continue };
            for category in value.field(META_CATEGORIES).and_then(items).unwrap_or(&[]) {
                for entry in category.field(META_ENTRIES).and_then(items).unwrap_or(&[]) {
                    let (Some(key), Some(text)) =
                        (entry.field(META_KEY).and_then(string), entry.field(META_VALUE).and_then(string))
                    else {
                        continue;
                    };
                    let text = text.trim();
                    if text.is_empty() {
                        continue;
                    }
                    match key {
                        "title" => title = title.or(Some(text.to_string())),
                        "author" => authors.push(text.to_string()),
                        "language" => language = language.or(Some(text.to_string())),
                        "ASIN" => asin = asin.or(Some(text.to_string())),
                        "publisher" => publisher = publisher.or(Some(text.to_string())),
                        "cover_image" => cover = cover.or(Some(text.to_string())),
                        _ => {}
                    }
                }
            }
        }
        let title = title.or_else(|| asin.clone()).unwrap_or_else(|| "Sin título".to_string());
        let meta = Meta { title, authors, language: language.unwrap_or_else(|| "und".to_string()), asin, publisher };
        (meta, cover)
    }

    /// Da ancla solo a los bloques a los que apunta algún enlace o el índice.
    fn place_anchors(&mut self) {
        for (chapter, positions) in self.chapters.iter_mut().zip(&self.block_positions) {
            for (block, position) in chapter.blocks.iter_mut().zip(positions) {
                let Some(p) = position.filter(|p| self.referenced.contains(p)) else { continue };
                let (Block::Heading { anchor, .. } | Block::Paragraph { anchor, .. } | Block::Image { anchor, .. }) =
                    block;
                *anchor = Some(format!("p{p}"));
            }
        }
    }
}

/// Parte el texto en tramos de formato uniforme. Las posiciones son puntos de código.
fn split_spans(text: &str, events: &[Event]) -> Vec<Span> {
    let chars: Vec<char> = text.chars().collect();
    let mut cuts: Vec<usize> = vec![0, chars.len()];
    for e in events {
        cuts.push(e.start);
        cuts.push(e.end);
    }
    cuts.sort_unstable();
    cuts.dedup();

    let mut spans = Vec::new();
    for pair in cuts.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        let covering = || events.iter().filter(move |e| e.start <= start && end <= e.end);
        spans.push(Span {
            text: chars[start..end].iter().collect(),
            bold: covering().any(|e| e.bold),
            italic: covering().any(|e| e.italic),
            link: covering().find_map(|e| e.link).map(str::to_string),
        });
    }
    merge_equal_spans(&mut spans);
    spans
}

/// Une tramos contiguos con el mismo formato.
fn merge_equal_spans(spans: &mut Vec<Span>) {
    let mut merged: Vec<Span> = Vec::with_capacity(spans.len());
    for span in spans.drain(..) {
        match merged.last_mut() {
            Some(last) if last.bold == span.bold && last.italic == span.italic && last.link == span.link => {
                last.text.push_str(&span.text);
            }
            _ => merged.push(span),
        }
    }
    *spans = merged;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ion::Ion;
    use crate::testkit::*;

    /// Un libro de prueba en construcción.
    struct T {
        b: BookBuilder,
    }

    fn with(node: Ion, extra: Vec<(u32, Ion)>) -> Ion {
        match node {
            Ion::Struct(mut fields) => {
                fields.extend(extra);
                Ion::Struct(fields)
            }
            other => other,
        }
    }

    fn measure(value: f64, unit: u32) -> Ion {
        st(vec![(VALUE, Ion::Float(value)), (UNIT, sym(unit))])
    }

    fn event(offset: i64, length: i64, extra: Vec<(u32, Ion)>) -> Ion {
        with(st(vec![(OFFSET, int(offset)), (LENGTH, int(length))]), extra)
    }

    impl T {
        fn new() -> Self {
            T { b: BookBuilder::new() }
        }

        fn content(&mut self, name: &str, texts: &[&str]) {
            let name_sym = self.b.lsym(name);
            let value = st(vec![(NAME, name_sym), (CONTENT_LIST, list(texts.iter().map(|t| s(t)).collect()))]);
            self.b.add(F_CONTENT, name, value);
        }

        fn text(&mut self, eid: i64, content: &str, index: i64) -> Ion {
            let reference = st(vec![(NAME, self.b.lsym(content)), (INDEX, int(index))]);
            st(vec![(POSITION_ID, int(eid)), (NODE_TYPE, sym(NODE_TEXT)), (CONTENT, reference)])
        }

        fn container(&mut self, eid: i64, children: Vec<Ion>) -> Ion {
            st(vec![(POSITION_ID, int(eid)), (NODE_TYPE, sym(NODE_CONTAINER)), (CONTENT_LIST, list(children))])
        }

        fn image(&mut self, eid: i64, resource: &str, alt: &str) -> Ion {
            st(vec![
                (POSITION_ID, int(eid)),
                (NODE_TYPE, sym(NODE_IMAGE)),
                (RESOURCE_NAME, self.b.lsym(resource)),
                (ALT_TEXT, s(alt)),
            ])
        }

        fn styled(&mut self, node: Ion, style: &str) -> Ion {
            let style_sym = self.b.lsym(style);
            with(node, vec![(STYLE, style_sym)])
        }

        fn style(&mut self, name: &str, props: Vec<(u32, Ion)>) {
            let mut fields = vec![(STYLE_NAME, self.b.lsym(name))];
            fields.extend(props);
            self.b.add(F_STYLE, name, st(fields));
        }

        fn bold_style(&mut self, name: &str) {
            self.style(name, vec![(FONT_WEIGHT, sym(361))]);
        }

        fn italic_style(&mut self, name: &str) {
            self.style(name, vec![(FONT_STYLE, sym(ITALIC))]);
        }

        fn section(&mut self, section: &str, story: &str, nodes: Vec<Ion>) {
            let template = st(vec![(POSITION_ID, int(9000)), (STORY_NAME, self.b.lsym(story)), (NODE_TYPE, sym(NODE_TEXT))]);
            let sec = st(vec![(SECTION_NAME, self.b.lsym(section)), (PAGE_TEMPLATES, list(vec![template]))]);
            self.b.add(F_SECTION, section, sec);
            let story_value = st(vec![(STORY_NAME, self.b.lsym(story)), (CONTENT_LIST, list(nodes))]);
            self.b.add(F_STORYLINE, story, story_value);
        }

        fn order(&mut self, sections: &[&str]) {
            let names: Vec<Ion> = sections.iter().map(|n| self.b.lsym(n)).collect();
            let order = st(vec![(178, sym(351)), (SECTIONS, list(names))]);
            self.b.add(F_DOCUMENT, F_DOCUMENT, st(vec![(READING_ORDERS, list(vec![order]))]));
        }

        fn anchor(&mut self, name: &str, eid: i64) {
            let value = st(vec![
                (ANCHOR_NAME, self.b.lsym(name)),
                (POSITION, st(vec![(POSITION_ID, int(eid)), (OFFSET, int(0))])),
            ]);
            self.b.add(F_ANCHOR, name, value);
        }

        fn resource(&mut self, name: &str, mime: &str, location: &str) {
            let value = st(vec![(RESOURCE_NAME, self.b.lsym(name)), (MIME, s(mime)), (LOCATION, s(location))]);
            self.b.add(F_RESOURCE, name, value);
        }

        fn metadata(&mut self, entries: &[(&str, &str)]) {
            let items: Vec<Ion> =
                entries.iter().map(|(k, v)| st(vec![(META_KEY, s(k)), (META_VALUE, s(v))])).collect();
            let category = st(vec![(META_CATEGORY, s("kindle_title_metadata")), (META_ENTRIES, list(items))]);
            self.b.add(F_BOOK_METADATA, F_BOOK_METADATA, st(vec![(META_CATEGORIES, list(vec![category]))]));
        }

        fn toc(&mut self, entries: Vec<Ion>) {
            let toc = Ion::Annotated(vec![391], Box::new(st(vec![(NAV_TYPE, sym(NAV_TOC)), (NAV_ENTRIES, list(entries))])));
            let headings = Ion::Annotated(vec![391], Box::new(st(vec![(NAV_TYPE, sym(798)), (NAV_ENTRIES, list(vec![]))])));
            let nav = st(vec![(178, sym(351)), (NAV_CONTAINERS, list(vec![headings, toc]))]);
            self.b.add(F_NAVIGATION, F_NAVIGATION, list(vec![nav]));
        }

        fn build(&self) -> Result<Book, Error> {
            build(&self.b.fragments)
        }

        /// Un libro mínimo: una sección con los textos dados, uno por párrafo.
        fn simple(texts: &[&str]) -> T {
            let mut t = T::new();
            t.content("k", texts);
            let nodes = (0..texts.len() as i64).map(|i| t.text(10 + i, "k", i)).collect();
            t.section("c0", "l0", nodes);
            t.order(&["c0"]);
            t
        }
    }

    fn toc_entry(label: &str, eid: i64, children: Vec<Ion>) -> Ion {
        Ion::Annotated(
            vec![393],
            Box::new(st(vec![
                (NAV_REPRESENTATION, st(vec![(NAV_LABEL, s(label))])),
                (NAV_TARGET, st(vec![(POSITION_ID, int(eid)), (OFFSET, int(0))])),
                (NAV_ENTRIES, list(children)),
            ])),
        )
    }

    fn spans_of(block: &Block) -> &[Span] {
        match block {
            Block::Heading { spans, .. } | Block::Paragraph { spans, .. } => spans,
            Block::Image { .. } => &[],
        }
    }

    fn text_of(block: &Block) -> String {
        spans_of(block).iter().map(|s| s.text.as_str()).collect()
    }

    fn anchor_of(block: &Block) -> Option<&str> {
        match block {
            Block::Heading { anchor, .. } | Block::Paragraph { anchor, .. } | Block::Image { anchor, .. } => {
                anchor.as_deref()
            }
        }
    }

    fn flags(block: &Block) -> Vec<(&str, bool, bool)> {
        spans_of(block).iter().map(|s| (s.text.as_str(), s.bold, s.italic)).collect()
    }

    #[test]
    fn chapters_follow_the_reading_order_not_the_fragment_order() {
        let mut t = T::new();
        t.content("k", &["Uno", "Dos"]);
        let second = t.text(20, "k", 1);
        t.section("c1", "l1", vec![second]);
        let first = t.text(10, "k", 0);
        t.section("c0", "l0", vec![first]);
        t.order(&["c0", "c1"]);

        let book = t.build().unwrap();

        let ids: Vec<&str> = book.chapters.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["c0", "c1"]);
        assert_eq!(text_of(&book.chapters[0].blocks[0]), "Uno");
        assert_eq!(text_of(&book.chapters[1].blocks[0]), "Dos");
        assert!(book.warnings.is_empty(), "{:?}", book.warnings);
    }

    #[test]
    fn no_reading_order_is_an_error() {
        let mut t = T::new();
        t.content("k", &["Uno"]);
        let node = t.text(10, "k", 0);
        t.section("c0", "l0", vec![node]);
        assert!(matches!(t.build(), Err(Error::NoReadingOrder)));

        t.order(&[]);
        assert!(matches!(t.build(), Err(Error::NoReadingOrder)));
    }

    #[test]
    fn section_missing_from_the_book_is_a_warning() {
        let mut t = T::simple(&["Uno"]);
        t.b.fragments.items.retain(|f| f.ftype != F_DOCUMENT);
        t.order(&["c0", "ghost"]);

        let book = t.build().unwrap();

        assert_eq!(book.chapters.len(), 1);
        assert_eq!(book.warnings.len(), 1);
        assert!(book.warnings[0].contains("ghost"), "{:?}", book.warnings);
    }

    #[test]
    fn unknown_node_type_is_skipped_with_a_warning() {
        let mut t = T::new();
        t.content("k", &["Uno", "Dos"]);
        let a = t.text(10, "k", 0);
        let odd = st(vec![(POSITION_ID, int(11)), (NODE_TYPE, sym(999))]);
        let b = t.text(12, "k", 1);
        t.section("c0", "l0", vec![a, odd, b]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        let texts: Vec<String> = book.chapters[0].blocks.iter().map(text_of).collect();
        assert_eq!(texts, vec!["Uno", "Dos"]);
        assert_eq!(book.warnings.len(), 1);
        assert!(book.warnings[0].contains("$999"), "{:?}", book.warnings);
    }

    #[test]
    fn text_under_an_unknown_node_type_is_kept() {
        let mut t = T::new();
        t.content("k", &["Uno", "Dos"]);
        let a = t.text(10, "k", 0);
        let item = t.text(12, "k", 1);
        let list_like = st(vec![(POSITION_ID, int(11)), (NODE_TYPE, sym(999)), (CONTENT_LIST, list(vec![item]))]);
        t.section("c0", "l0", vec![a, list_like]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        let texts: Vec<String> = book.chapters[0].blocks.iter().map(text_of).collect();
        assert_eq!(texts, vec!["Uno", "Dos"], "se pierde la estructura, no las palabras");
        assert_eq!(book.warnings.len(), 1);
    }

    #[test]
    fn title_can_be_read_on_its_own() {
        let mut t = T::simple(&["Uno"]);
        assert_eq!(title(&t.b.fragments), None);
        t.metadata(&[("title", "Mi libro")]);
        assert_eq!(title(&t.b.fragments).as_deref(), Some("Mi libro"));
    }

    #[test]
    fn missing_text_is_a_warning_not_an_error() {
        let mut t = T::new();
        t.content("k", &["Uno"]);
        let ok = t.text(10, "k", 0);
        let no_fragment = t.text(11, "nope", 0);
        let no_index = t.text(12, "k", 7);
        t.section("c0", "l0", vec![ok, no_fragment, no_index]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        assert_eq!(book.chapters[0].blocks.len(), 1);
        assert_eq!(book.warnings.len(), 2, "{:?}", book.warnings);
    }

    #[test]
    fn empty_text_produces_no_block_and_no_warning() {
        let book = T::simple(&["Uno", "", "Tres"]).build().unwrap();
        assert_eq!(book.chapters[0].blocks.len(), 2);
        assert!(book.warnings.is_empty());
    }

    #[test]
    fn nested_containers_flatten_in_order() {
        let mut t = T::new();
        t.content("k", &["A", "B", "C"]);
        let a = t.text(10, "k", 0);
        let b = t.text(11, "k", 1);
        let inner = t.container(21, vec![b]);
        let outer = t.container(20, vec![a, inner]);
        let c = t.text(12, "k", 2);
        t.section("c0", "l0", vec![outer, c]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        let texts: Vec<String> = book.chapters[0].blocks.iter().map(text_of).collect();
        assert_eq!(texts, vec!["A", "B", "C"]);
    }

    #[test]
    fn heading_level_makes_a_heading() {
        let mut t = T::new();
        t.content("k", &["Título", "Demasiado profundo", "Texto"]);
        let h = with(t.text(10, "k", 0), vec![(HEADING_LEVEL, int(2))]);
        let deep = with(t.text(11, "k", 1), vec![(HEADING_LEVEL, int(9))]);
        let p = t.text(12, "k", 2);
        t.section("c0", "l0", vec![h, deep, p]);
        t.order(&["c0"]);

        let blocks = t.build().unwrap().chapters.remove(0).blocks;

        assert!(matches!(&blocks[0], Block::Heading { level: 2, .. }));
        assert!(matches!(&blocks[1], Block::Heading { level: 6, .. }));
        assert!(matches!(&blocks[2], Block::Paragraph { .. }));
    }

    #[test]
    fn style_events_split_text_by_code_points() {
        let mut t = T::new();
        t.bold_style("sb");
        t.italic_style("si");
        t.content("k", &["añö—é fin"]);
        let bold = event(0, 5, vec![(STYLE, t.b.lsym("sb"))]);
        let italic = event(6, 3, vec![(STYLE, t.b.lsym("si"))]);
        let node = with(t.text(10, "k", 0), vec![(STYLE_EVENTS, list(vec![bold, italic]))]);
        t.section("c0", "l0", vec![node]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        assert_eq!(
            flags(&book.chapters[0].blocks[0]),
            vec![("añö—é", true, false), (" ", false, false), ("fin", false, true)]
        );
    }

    #[test]
    fn overlapping_events_combine() {
        let mut t = T::new();
        t.bold_style("sb");
        t.italic_style("si");
        t.content("k", &["abcdefgh"]);
        let bold = event(0, 5, vec![(STYLE, t.b.lsym("sb"))]);
        let italic = event(3, 5, vec![(STYLE, t.b.lsym("si"))]);
        let node = with(t.text(10, "k", 0), vec![(STYLE_EVENTS, list(vec![bold, italic]))]);
        t.section("c0", "l0", vec![node]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        assert_eq!(
            flags(&book.chapters[0].blocks[0]),
            vec![("abc", true, false), ("de", true, true), ("fgh", false, true)]
        );
    }

    #[test]
    fn events_past_the_end_are_clamped() {
        let mut t = T::new();
        t.bold_style("sb");
        t.content("k", &["abc"]);
        let long = event(1, 99, vec![(STYLE, t.b.lsym("sb"))]);
        let beyond = event(50, 2, vec![(STYLE, t.b.lsym("sb"))]);
        let negative = event(-4, 2, vec![(STYLE, t.b.lsym("sb"))]);
        let node = with(t.text(10, "k", 0), vec![(STYLE_EVENTS, list(vec![long, beyond, negative]))]);
        t.section("c0", "l0", vec![node]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        assert_eq!(flags(&book.chapters[0].blocks[0]), vec![("a", false, false), ("bc", true, false)]);
    }

    #[test]
    fn event_with_an_unstyled_style_leaves_plain_text_whole() {
        let mut t = T::new();
        t.style("plain", vec![]);
        t.content("k", &["abcdef"]);
        let ev = event(2, 2, vec![(STYLE, t.b.lsym("plain"))]);
        let node = with(t.text(10, "k", 0), vec![(STYLE_EVENTS, list(vec![ev]))]);
        t.section("c0", "l0", vec![node]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        assert_eq!(flags(&book.chapters[0].blocks[0]), vec![("abcdef", false, false)]);
    }

    #[test]
    fn links_resolve_to_chapter_and_anchor() {
        let mut t = T::new();
        t.content("k", &["ver capítulo", "Destino", "Otro"]);
        t.anchor("a1", 20);
        let link = event(4, 8, vec![(LINK_TO, t.b.lsym("a1"))]);
        let source = with(t.text(10, "k", 0), vec![(STYLE_EVENTS, list(vec![link]))]);
        t.section("c0", "l0", vec![source]);
        let target = t.text(20, "k", 1);
        let other = t.text(21, "k", 2);
        t.section("c1", "l1", vec![target, other]);
        t.order(&["c0", "c1"]);

        let book = t.build().unwrap();

        let spans = spans_of(&book.chapters[0].blocks[0]);
        assert_eq!(spans.len(), 2);
        assert_eq!((spans[0].text.as_str(), spans[0].link.as_deref()), ("ver ", None));
        assert_eq!((spans[1].text.as_str(), spans[1].link.as_deref()), ("capítulo", Some("c1#p20")));
        assert_eq!(anchor_of(&book.chapters[1].blocks[0]), Some("p20"));
        assert_eq!(anchor_of(&book.chapters[1].blocks[1]), None, "un bloque al que nadie apunta no lleva ancla");
        assert_eq!(anchor_of(&book.chapters[0].blocks[0]), None);
    }

    #[test]
    fn link_to_an_unknown_anchor_is_dropped_with_a_warning() {
        let mut t = T::new();
        t.content("k", &["ver nada"]);
        let link = event(4, 4, vec![(LINK_TO, t.b.lsym("ghost"))]);
        let node = with(t.text(10, "k", 0), vec![(STYLE_EVENTS, list(vec![link]))]);
        t.section("c0", "l0", vec![node]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        assert!(spans_of(&book.chapters[0].blocks[0]).iter().all(|s| s.link.is_none()));
        assert_eq!(text_of(&book.chapters[0].blocks[0]), "ver nada");
        assert_eq!(book.warnings.len(), 1, "{:?}", book.warnings);
    }

    #[test]
    fn images_carry_their_bytes_and_type() {
        let mut t = T::new();
        t.resource("e1", "image/jpg", "resource/r1");
        t.b.add_raw(F_RAW_MEDIA, "resource/r1", vec![0xFF, 0xD8, 0xFF, 0xE0]);
        let img = t.image(10, "e1", "Mapa del reino");
        t.section("c0", "l0", vec![img]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        match &book.chapters[0].blocks[0] {
            Block::Image { name, alt, .. } => {
                assert_eq!(name, "e1.jpg");
                assert_eq!(alt, "Mapa del reino");
            }
            other => panic!("se esperaba una imagen, llegó {other:?}"),
        }
        assert_eq!(
            book.images,
            vec![Image { name: "e1.jpg".into(), mime: "image/jpeg".into(), data: vec![0xFF, 0xD8, 0xFF, 0xE0] }]
        );
    }

    #[test]
    fn the_same_image_used_twice_is_stored_once() {
        let mut t = T::new();
        t.resource("e1", "image/png", "resource/r1");
        t.b.add_raw(F_RAW_MEDIA, "resource/r1", vec![1, 2, 3]);
        let a = t.image(10, "e1", "");
        let b = t.image(11, "e1", "");
        t.section("c0", "l0", vec![a, b]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        assert_eq!(book.chapters[0].blocks.len(), 2);
        assert_eq!(book.images.len(), 1);
        assert_eq!(book.images[0].name, "e1.png");
    }

    #[test]
    fn image_without_data_is_a_warning() {
        let mut t = T::new();
        t.resource("e1", "image/jpg", "resource/missing");
        let img = t.image(10, "e1", "");
        let ghost = t.image(11, "no-resource", "");
        t.section("c0", "l0", vec![img, ghost]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        assert!(book.chapters[0].blocks.is_empty());
        assert!(book.images.is_empty());
        assert_eq!(book.warnings.len(), 2, "{:?}", book.warnings);
    }

    #[test]
    fn cover_comes_from_metadata_even_if_no_page_shows_it() {
        let mut t = T::simple(&["Uno"]);
        t.resource("e9", "image/jpg", "resource/cover");
        t.b.add_raw(F_RAW_MEDIA, "resource/cover", vec![9, 9]);
        t.metadata(&[("title", "Mi libro"), ("cover_image", "e9")]);

        let book = t.build().unwrap();

        assert_eq!(book.cover.as_deref(), Some("e9.jpg"));
        assert_eq!(book.images.len(), 1);
        assert_eq!(book.images[0].name, "e9.jpg");
    }

    #[test]
    fn metadata_fields_are_read() {
        let mut t = T::simple(&["Uno"]);
        t.metadata(&[
            ("title", "Mi libro"),
            ("author", "Ana Pérez"),
            ("author", "Luis Gómez"),
            ("language", "es"),
            ("ASIN", "B0TEST"),
            ("publisher", "Editorial X"),
        ]);

        let meta = t.build().unwrap().meta;

        assert_eq!(meta.title, "Mi libro");
        assert_eq!(meta.authors, vec!["Ana Pérez", "Luis Gómez"]);
        assert_eq!(meta.language, "es");
        assert_eq!(meta.asin.as_deref(), Some("B0TEST"));
        assert_eq!(meta.publisher.as_deref(), Some("Editorial X"));
    }

    #[test]
    fn missing_title_falls_back_to_the_asin_then_to_a_placeholder() {
        let mut with_asin = T::simple(&["Uno"]);
        with_asin.metadata(&[("ASIN", "B0TEST")]);
        let meta = with_asin.build().unwrap().meta;
        assert_eq!(meta.title, "B0TEST");
        assert_eq!(meta.language, "und");
        assert!(meta.authors.is_empty());

        let bare = T::simple(&["Uno"]).build().unwrap().meta;
        assert_eq!(bare.title, "Sin título");
        assert_eq!(bare.asin, None);
    }

    #[test]
    fn toc_entries_nest_and_point_at_blocks() {
        let mut t = T::new();
        t.content("k", &["Parte I", "Capítulo 1", "Texto"]);
        let part = t.text(10, "k", 0);
        t.section("c0", "l0", vec![part]);
        let chapter = t.text(20, "k", 1);
        let body = t.text(21, "k", 2);
        t.section("c1", "l1", vec![chapter, body]);
        t.order(&["c0", "c1"]);
        t.toc(vec![toc_entry("Parte I", 10, vec![toc_entry("Capítulo 1", 20, vec![])])]);

        let book = t.build().unwrap();

        assert_eq!(
            book.toc,
            vec![TocEntry {
                label: "Parte I".into(),
                target: "c0#p10".into(),
                children: vec![TocEntry { label: "Capítulo 1".into(), target: "c1#p20".into(), children: vec![] }],
            }]
        );
        assert_eq!(anchor_of(&book.chapters[0].blocks[0]), Some("p10"));
        assert_eq!(anchor_of(&book.chapters[1].blocks[0]), Some("p20"));
        assert_eq!(anchor_of(&book.chapters[1].blocks[1]), None);
    }

    #[test]
    fn toc_target_on_a_container_lands_on_its_first_block() {
        let mut t = T::new();
        t.content("k", &["Dentro"]);
        let inner = t.text(11, "k", 0);
        let outer = t.container(10, vec![inner]);
        t.section("c0", "l0", vec![outer]);
        t.order(&["c0"]);
        t.toc(vec![toc_entry("Inicio", 10, vec![])]);

        let book = t.build().unwrap();

        assert_eq!(book.toc[0].target, "c0#p11");
        assert_eq!(anchor_of(&book.chapters[0].blocks[0]), Some("p11"));
    }

    #[test]
    fn toc_entry_with_an_unknown_target_is_dropped_but_keeps_its_children() {
        let mut t = T::simple(&["Uno"]);
        t.toc(vec![toc_entry("Fantasma", 777, vec![toc_entry("Real", 10, vec![])])]);

        let book = t.build().unwrap();

        assert_eq!(book.toc, vec![TocEntry { label: "Real".into(), target: "c0#p10".into(), children: vec![] }]);
        assert_eq!(book.warnings.len(), 1, "{:?}", book.warnings);
    }

    #[test]
    fn book_without_navigation_has_an_empty_toc() {
        let book = T::simple(&["Uno"]).build().unwrap();
        assert!(book.toc.is_empty());
        assert!(book.warnings.is_empty());
    }

    #[test]
    fn block_styles_are_read() {
        let mut t = T::new();
        t.style(
            "s1",
            vec![
                (TEXT_ALIGN, sym(ALIGN_CENTER)),
                (TEXT_INDENT, measure(1.5, UNIT_EM)),
                (MARGIN_TOP, measure(2.0, UNIT_LH)),
                (MARGIN_BOTTOM, measure(10.0, UNIT_PERCENT)),
                (FONT_SIZE, measure(1.6, UNIT_REM)),
                (FONT_STYLE, sym(ITALIC)),
                (FONT_WEIGHT, sym(361)),
            ],
        );
        t.content("k", &["Centrado"]);
        let node = t.text(10, "k", 0);
        let node = t.styled(node, "s1");
        t.section("c0", "l0", vec![node]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        let Block::Paragraph { style, .. } = &book.chapters[0].blocks[0] else { panic!("párrafo") };
        assert_eq!(style.align, Some(Align::Center));
        assert_eq!(style.indent, Some(Length::Em(1.5)));
        assert!(matches!(style.margin_top, Some(Length::Em(v)) if (v - 2.4).abs() < 1e-9), "{:?}", style.margin_top);
        assert_eq!(style.margin_bottom, Some(Length::Percent(10.0)));
        assert_eq!(style.font_size, Some(Length::Rem(1.6)));
        assert!(style.italic && style.bold);
    }

    #[test]
    fn external_links_keep_their_url() {
        let mut t = T::new();
        t.content("k", &["visita la web"]);
        let anchor = st(vec![(ANCHOR_NAME, t.b.lsym("a1")), (ANCHOR_URI, s("https://example.org/libro"))]);
        t.b.add(F_ANCHOR, "a1", anchor);
        let link = event(10, 3, vec![(LINK_TO, t.b.lsym("a1"))]);
        let node = with(t.text(10, "k", 0), vec![(STYLE_EVENTS, list(vec![link]))]);
        t.section("c0", "l0", vec![node]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        let spans = spans_of(&book.chapters[0].blocks[0]);
        assert_eq!(spans[1].text, "web");
        assert_eq!(spans[1].link.as_deref(), Some("https://example.org/libro"));
        assert!(book.warnings.is_empty(), "{:?}", book.warnings);
    }

    #[test]
    fn unknown_style_and_default_values_give_an_empty_style() {
        let mut t = T::new();
        t.style("s1", vec![(TEXT_ALIGN, sym(12345)), (FONT_WEIGHT, sym(350)), (FONT_SIZE, measure(1.0, UNIT_REM))]);
        t.content("k", &["A", "B"]);
        let a = t.text(10, "k", 0);
        let a = t.styled(a, "s1");
        let b = t.text(11, "k", 1);
        let b = t.styled(b, "no-such-style");
        t.section("c0", "l0", vec![a, b]);
        t.order(&["c0"]);

        let book = t.build().unwrap();

        for block in &book.chapters[0].blocks {
            let Block::Paragraph { style, .. } = block else { panic!("párrafo") };
            assert_eq!(style, &Style::default());
        }
    }
}
