use cycms_core::Result;
use cycms_host_types::ContentNode;

use crate::content_ir::{ContentInput, ContentParser, ParseContext, build_document, invalid_input};

pub struct HtmlContentParser;

impl ContentParser for HtmlContentParser {
    fn parse(
        &self,
        input: ContentInput,
        ctx: &ParseContext,
    ) -> Result<cycms_host_types::ContentDocument> {
        let ContentInput::Text(html) = input else {
            return Err(invalid_input("text", &input));
        };
        Ok(build_document(
            ctx,
            vec![ContentNode::RawHtml(html)],
            Vec::new(),
        ))
    }
}
