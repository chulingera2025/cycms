pub mod blocks;
pub mod html;
pub mod markdown;
pub mod tiptap;

use cycms_core::{Error, Result};
use cycms_host_types::ContentDocument;

use crate::content_ir::{ContentInput, ContentParser, ParseContext};

pub use blocks::BlockJsonParser;
pub use html::HtmlContentParser;
pub use markdown::MarkdownParser;
pub use tiptap::TiptapJsonParser;

pub fn parse_with_defaults(input: ContentInput, ctx: &ParseContext) -> Result<ContentDocument> {
    match ctx.format.as_str() {
        "markdown" | "md" => MarkdownParser.parse(input, ctx),
        "tiptap" | "tiptap_json" => TiptapJsonParser.parse(input, ctx),
        "html" => HtmlContentParser.parse(input, ctx),
        "blocks" | "block_json" => BlockJsonParser.parse(input, ctx),
        other => Err(Error::ValidationError {
            message: format!("unsupported content format {other:?}"),
            details: None,
        }),
    }
}
