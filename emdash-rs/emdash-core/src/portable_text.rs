use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    #[serde(rename = "_type")]
    pub doc_type: String,
    #[serde(rename = "_key")]
    pub key: String,
    pub text: String,
    pub marks: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    #[serde(rename = "_type")]
    pub doc_type: String,
    #[serde(rename = "_key")]
    pub key: String,
    pub children: Vec<Span>,
    pub style: Option<String>,
    #[serde(rename = "markDefs")]
    pub mark_defs: Option<Vec<serde_json::Value>>,
}
