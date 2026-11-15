//! Names for the `YJ_symbols` symbols the converter uses.
//!
//! In KFX, fields and values are numbered symbols (`$155`, `$260`, ...). Here they are
//! named after what they do. Format reference: `kfxlib` from John Howell's KFX Input
//! plugin (GPL-3.0), checked against real books.

/// The `name` field of the system table; in a text reference it names its fragment.
pub const NAME: u32 = 4;

// Fragment types.
pub const F_CONTENT: &str = "$145";
pub const F_STYLE: &str = "$157";
pub const F_RESOURCE: &str = "$164";
pub const F_METADATA_OLD: &str = "$258";
pub const F_STORYLINE: &str = "$259";
pub const F_SECTION: &str = "$260";
pub const F_ANCHOR: &str = "$266";
pub const F_NAVIGATION: &str = "$389";
pub const F_RAW_MEDIA: &str = "$417";
pub const F_BOOK_METADATA: &str = "$490";
pub const F_DOCUMENT: &str = "$538";

// Style properties.
pub const LANGUAGE: u32 = 10;
pub const FONT_STYLE: u32 = 12;
pub const FONT_WEIGHT: u32 = 13;
pub const FONT_SIZE: u32 = 16;
pub const TEXT_ALIGN: u32 = 34;
pub const TEXT_INDENT: u32 = 36;
pub const MARGIN_TOP: u32 = 47;
pub const MARGIN_BOTTOM: u32 = 49;

// Style values.
pub const ALIGN_LEFT: u32 = 59;
pub const ALIGN_RIGHT: u32 = 61;
pub const ALIGN_CENTER: u32 = 320;
pub const ALIGN_JUSTIFY: u32 = 321;
pub const ITALIC: u32 = 382;
/// Weights from semibold (600) up: `$360`..=`$363`.
pub const WEIGHT_BOLD_FROM: u32 = 360;
pub const WEIGHT_BOLD_TO: u32 = 363;

// Measures: `{ $307: value, $306: unit }`.
pub const UNIT: u32 = 306;
pub const VALUE: u32 = 307;
pub const UNIT_EM: u32 = 308;
pub const UNIT_LH: u32 = 310;
pub const UNIT_PERCENT: u32 = 314;
pub const UNIT_PT: u32 = 318;
pub const UNIT_REM: u32 = 505;

// Sections, storylines and content nodes.
pub const PAGE_TEMPLATES: u32 = 141;
pub const STYLE_EVENTS: u32 = 142;
pub const OFFSET: u32 = 143;
pub const LENGTH: u32 = 144;
pub const CONTENT: u32 = 145;
pub const CONTENT_LIST: u32 = 146;
pub const POSITION_ID: u32 = 155;
pub const STYLE: u32 = 157;
pub const NODE_TYPE: u32 = 159;
pub const STYLE_NAME: u32 = 173;
pub const SECTION_NAME: u32 = 174;
pub const RESOURCE_NAME: u32 = 175;
pub const STORY_NAME: u32 = 176;
pub const LINK_TO: u32 = 179;
pub const INDEX: u32 = 403;
pub const ALT_TEXT: u32 = 584;
pub const HEADING_LEVEL: u32 = 790;

// Node types.
pub const NODE_TEXT: u32 = 269;
pub const NODE_CONTAINER: u32 = 270;
pub const NODE_IMAGE: u32 = 271;

// Resources.
pub const MIME: u32 = 162;
pub const LOCATION: u32 = 165;

// Reading order.
pub const READING_ORDERS: u32 = 169;
pub const SECTIONS: u32 = 170;

// Anchors.
pub const ANCHOR_NAME: u32 = 180;
pub const POSITION: u32 = 183;

// Navigation.
pub const NAV_TYPE: u32 = 235;
pub const NAV_REPRESENTATION: u32 = 241;
pub const NAV_LABEL: u32 = 244;
pub const NAV_TARGET: u32 = 246;
pub const NAV_ENTRIES: u32 = 247;
pub const NAV_CONTAINERS: u32 = 392;
pub const NAV_TOC: u32 = 212;

// Metadata.
pub const META_VALUE: u32 = 307;
pub const META_ENTRIES: u32 = 258;
pub const META_CATEGORIES: u32 = 491;
pub const META_KEY: u32 = 492;
pub const META_CATEGORY: u32 = 495;

// Anchors with an external target.
pub const ANCHOR_URI: u32 = 186;
