use cycms_core::{Error, Result};
use cycms_host_types::{BlockNode, ContentNode, EmbedNode, HeadingNode, ParagraphNode};
use serde_json::Value;

use crate::content_ir::{ContentInput, ContentParser, ParseContext, build_document, invalid_input};

pub struct TiptapJsonParser;

impl ContentParser for TiptapJsonParser {
    fn parse(
        &self,
        input: ContentInput,
        ctx: &ParseContext,
    ) -> Result<cycms_host_types::ContentDocument> {
        let ContentInput::Json(value) = input else {
            return Err(invalid_input("json", &input));
        };

        let content = value
            .get("content")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::ValidationError {
                message: "tiptap document must contain a content array".to_owned(),
                details: None,
            })?;

        let mut nodes = Vec::new();
        for node in content {
            match node.get("type").and_then(Value::as_str).unwrap_or_default() {
                "heading" => {
                    let level = node
                        .get("attrs")
                        .and_then(|attrs| attrs.get("level"))
                        .and_then(Value::as_u64)
                        .unwrap_or(1) as u8;
                    nodes.push(ContentNode::Heading(HeadingNode {
                        level,
                        text: extract_text(node),
                    }));
                }
                "paragraph" => {
                    let text = extract_text(node);
                    if !text.is_empty() {
                        nodes.push(ContentNode::Paragraph(ParagraphNode { text }));
                    }
                }
                "image" => {
                    let src = node
                        .get("attrs")
                        .and_then(|attrs| attrs.get("src"))
                        .and_then(Value::as_str)
                        .ok_or_else(|| Error::ValidationError {
                            message: "tiptap image node must include attrs.src".to_owned(),
                            details: None,
                        })?;
                    let title = node
                        .get("attrs")
                        .and_then(|attrs| attrs.get("alt"))
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned);
                    nodes.push(ContentNode::Embed(EmbedNode {
                        url: src.to_owned(),
                        title,
                    }));
                }
                other => nodes.push(ContentNode::Block(BlockNode {
                    kind: if other.is_empty() {
                        "unknown".to_owned()
                    } else {
                        other.to_owned()
                    },
                    data: node.clone(),
                })),
            }
        }

        Ok(build_document(ctx, nodes, Vec::new()))
    }
}

fn extract_text(node: &Value) -> String {
    match node.get("type").and_then(Value::as_str) {
        Some("text") => node
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        Some("hardBreak") => "\n".to_owned(),
        _ => node
            .get("content")
            .and_then(Value::as_array)
            .map(|children| children.iter().map(extract_text).collect::<String>())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::content_ir::ParseContext;

    #[test]
    fn parses_tiptap_heading_paragraph_and_image_nodes() {
        let ctx = ParseContext {
            format: "tiptap".to_owned(),
            parser_id: "default.tiptap".to_owned(),
            origin_field: Some("body".to_owned()),
            content_type: Some("post".to_owned()),
        };
        let document = TiptapJsonParser
            .parse(
                ContentInput::Json(json!({
                    "type": "doc",
                    "content": [
                        { "type": "heading", "attrs": { "level": 2 }, "content": [{ "type": "text", "text": "Hello" }] },
                        { "type": "paragraph", "content": [{ "type": "text", "text": "World" }] },
                        { "type": "image", "attrs": { "src": "https://example.com/cat.png", "alt": "cat" } }
                    ]
                })),
                &ctx,
            )
            .unwrap();

        assert_eq!(document.nodes.len(), 3);
        assert!(matches!(document.nodes[0], ContentNode::Heading(_)));
        assert!(matches!(document.nodes[1], ContentNode::Paragraph(_)));
        assert!(matches!(document.nodes[2], ContentNode::Embed(_)));
    }
}
