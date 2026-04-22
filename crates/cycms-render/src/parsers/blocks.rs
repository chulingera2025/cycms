use cycms_core::{Error, Result};
use cycms_host_types::{BlockNode, ContentNode, EmbedNode, SlotCallNode};
use serde_json::Value;

use crate::content_ir::{ContentInput, ContentParser, ParseContext, build_document, invalid_input};

pub struct BlockJsonParser;

impl ContentParser for BlockJsonParser {
    fn parse(
        &self,
        input: ContentInput,
        ctx: &ParseContext,
    ) -> Result<cycms_host_types::ContentDocument> {
        let ContentInput::Json(value) = input else {
            return Err(invalid_input("json", &input));
        };
        let items = value.as_array().ok_or_else(|| Error::ValidationError {
            message: "block_json format expects a JSON array".to_owned(),
            details: None,
        })?;

        let mut nodes = Vec::new();
        for item in items {
            if let Some(slot) = item.get("slot").and_then(Value::as_str) {
                let arguments = item
                    .get("arguments")
                    .and_then(Value::as_object)
                    .map(|map| map.clone().into_iter().collect())
                    .unwrap_or_default();
                nodes.push(ContentNode::SlotCall(SlotCallNode {
                    slot: slot.to_owned(),
                    arguments,
                }));
                continue;
            }

            if let Some(url) = item.get("url").and_then(Value::as_str) {
                nodes.push(ContentNode::Embed(EmbedNode {
                    url: url.to_owned(),
                    title: item
                        .get("title")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                }));
                continue;
            }

            let kind = item
                .get("type")
                .or_else(|| item.get("kind"))
                .and_then(Value::as_str)
                .unwrap_or("block");
            nodes.push(ContentNode::Block(BlockNode {
                kind: kind.to_owned(),
                data: item.clone(),
            }));
        }

        Ok(build_document(ctx, nodes, Vec::new()))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::content_ir::ParseContext;

    #[test]
    fn parses_block_json_into_slot_embed_and_block_nodes() {
        let ctx = ParseContext {
            format: "blocks".to_owned(),
            parser_id: "default.blocks".to_owned(),
            origin_field: Some("body".to_owned()),
            content_type: Some("page".to_owned()),
        };
        let document = BlockJsonParser
            .parse(
                ContentInput::Json(json!([
                    { "slot": "hero", "arguments": { "variant": "landing" } },
                    { "url": "https://example.com/embed", "title": "Example" },
                    { "type": "quote", "text": "hello" }
                ])),
                &ctx,
            )
            .unwrap();

        assert_eq!(document.nodes.len(), 3);
        assert!(matches!(document.nodes[0], ContentNode::SlotCall(_)));
        assert!(matches!(document.nodes[1], ContentNode::Embed(_)));
        assert!(matches!(document.nodes[2], ContentNode::Block(_)));
    }
}
