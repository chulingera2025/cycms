use cycms_core::{Error, Result};
use cycms_host_types::{
    CONTENT_DOCUMENT_SCHEMA_VERSION, ContentDiagnostic, ContentDocument, ContentNode,
    ContentSourceMeta,
};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum ContentInput {
    Text(String),
    Json(Value),
}

impl ContentInput {
    #[must_use]
    pub const fn kind_label(&self) -> &'static str {
        match self {
            Self::Text(_) => "text",
            Self::Json(_) => "json",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseContext {
    pub format: String,
    pub parser_id: String,
    pub origin_field: Option<String>,
    pub content_type: Option<String>,
}

impl ParseContext {
    #[must_use]
    pub fn source_meta(&self) -> ContentSourceMeta {
        ContentSourceMeta {
            format: self.format.clone(),
            parser_id: self.parser_id.clone(),
            origin_field: self.origin_field.clone(),
            content_type: self.content_type.clone(),
        }
    }
}

pub trait ContentParser {
    fn parse(&self, input: ContentInput, ctx: &ParseContext) -> Result<ContentDocument>;
}

pub(crate) fn invalid_input(expected: &str, actual: &ContentInput) -> Error {
    Error::ValidationError {
        message: format!(
            "parser expected {expected} input, received {}",
            actual.kind_label()
        ),
        details: None,
    }
}

pub(crate) fn build_document(
    ctx: &ParseContext,
    nodes: Vec<ContentNode>,
    diagnostics: Vec<ContentDiagnostic>,
) -> ContentDocument {
    ContentDocument {
        schema_version: CONTENT_DOCUMENT_SCHEMA_VERSION,
        source: ctx.source_meta(),
        nodes,
        diagnostics,
    }
}
