/// Full PortableText AST matching the `@portabletext/toolkit` spec.
///
/// A document is `Vec<PortableTextNode>`.  Handlers serialise/deserialise
/// these via `serde_json` to/from the JSON column in the database.
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Mark definition (links, annotations) ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MarkDef {
    #[serde(rename = "_key")]
    pub key: String,
    #[serde(rename = "_type")]
    pub def_type: String,
    /// Arbitrary extra fields (e.g. `href`, `rel`).
    #[serde(flatten)]
    pub extra: Value,
}

// ── Inline span ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Span {
    #[serde(rename = "_type")]
    pub span_type: String, // always "span"
    #[serde(rename = "_key")]
    pub key: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<String>,
}

// ── Text block ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Block {
    #[serde(rename = "_type")]
    pub block_type: String, // usually "block"
    #[serde(rename = "_key")]
    pub key: String,
    pub style: Option<String>, // "normal" | "h1" … "h6" | "blockquote"
    pub children: Vec<Span>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mark_defs: Vec<MarkDef>,
    pub level: Option<u8>,
    pub list_item: Option<String>, // "bullet" | "number"
}

// ── Image block ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ImageAsset {
    pub id: String,
    pub url: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ImageBlock {
    #[serde(rename = "_type")]
    pub block_type: String, // "image"
    #[serde(rename = "_key")]
    pub key: String,
    pub asset: ImageAsset,
    pub alt: Option<String>,
    pub caption: Option<String>,
    pub hotspot: Option<Hotspot>,
    pub crop: Option<Crop>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Hotspot {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Crop {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

// ── Generic custom object block ───────────────────────────────────────────────

/// Catch-all for custom block types defined by plugins or collections.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ObjectBlock {
    #[serde(rename = "_type")]
    pub block_type: String,
    #[serde(rename = "_key")]
    pub key: String,
    #[serde(flatten)]
    pub fields: Value,
}

// ── Top-level union ───────────────────────────────────────────────────────────

/// Top-level PortableText node.
///
/// Uses `#[serde(untagged)]` so serde tries each variant in order; the
/// catch-all `Object` variant at the end accepts any remaining JSON object.
/// Serialisation re-emits the original `_type` field via `ObjectBlock::block_type`.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(untagged)]
pub enum PortableTextNode {
    Image(ImageBlock),
    Block(Block),
    /// Catch-all for plugin-defined or future block types.
    Object(ObjectBlock),
}

/// Convenience alias — a full PortableText document.
pub type PortableTextDoc = Vec<PortableTextNode>;
