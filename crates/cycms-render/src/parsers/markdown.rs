use cycms_core::Result;
use cycms_host_types::{
    BlockNode, ContentDiagnostic, ContentDiagnosticSeverity, ContentNode, HeadingNode,
    ParagraphNode,
};
use serde_json::json;

use crate::content_ir::{ContentInput, ContentParser, ParseContext, build_document, invalid_input};

pub struct MarkdownParser;

impl ContentParser for MarkdownParser {
    fn parse(
        &self,
        input: ContentInput,
        ctx: &ParseContext,
    ) -> Result<cycms_host_types::ContentDocument> {
        let ContentInput::Text(text) = input else {
            return Err(invalid_input("text", &input));
        };

        let mut nodes = Vec::new();
        let mut diagnostics = Vec::new();
        let mut paragraph = Vec::new();
        let mut fenced_language: Option<String> = None;
        let mut fenced_lines = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim_end();
            if let Some(language) = &fenced_language {
                if trimmed.starts_with("```") {
                    nodes.push(ContentNode::Block(BlockNode {
                        kind: "code_block".to_owned(),
                        data: json!({
                            "language": language,
                            "code": fenced_lines.join("\n"),
                        }),
                    }));
                    fenced_language = None;
                    fenced_lines.clear();
                } else {
                    fenced_lines.push(trimmed.to_owned());
                }
                continue;
            }

            if let Some(language) = trimmed.strip_prefix("```") {
                flush_paragraph(&mut paragraph, &mut nodes);
                fenced_language = Some(language.trim().to_owned());
                continue;
            }

            if trimmed.is_empty() {
                flush_paragraph(&mut paragraph, &mut nodes);
                continue;
            }

            if let Some((level, heading_text)) = parse_heading(trimmed) {
                flush_paragraph(&mut paragraph, &mut nodes);
                nodes.push(ContentNode::Heading(HeadingNode {
                    level,
                    text: heading_text.to_owned(),
                }));
                continue;
            }

            paragraph.push(trimmed.trim().to_owned());
        }

        if fenced_language.is_some() {
            diagnostics.push(ContentDiagnostic {
                severity: ContentDiagnosticSeverity::Warning,
                code: "markdown_unclosed_code_fence".to_owned(),
                message: "markdown code fence was not closed before EOF".to_owned(),
            });
            nodes.push(ContentNode::Block(BlockNode {
                kind: "code_block".to_owned(),
                data: json!({
                    "language": fenced_language.unwrap_or_default(),
                    "code": fenced_lines.join("\n"),
                }),
            }));
        }

        flush_paragraph(&mut paragraph, &mut nodes);

        Ok(build_document(ctx, nodes, diagnostics))
    }
}

fn flush_paragraph(paragraph: &mut Vec<String>, nodes: &mut Vec<ContentNode>) {
    if paragraph.is_empty() {
        return;
    }
    nodes.push(ContentNode::Paragraph(ParagraphNode {
        text: paragraph.join(" "),
    }));
    paragraph.clear();
}

fn parse_heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line.chars().take_while(|ch| *ch == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = line.get(hashes..)?.trim_start();
    if rest.is_empty() {
        return None;
    }
    Some((hashes as u8, rest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_ir::ParseContext;

    #[test]
    fn parses_markdown_headings_paragraphs_and_code_blocks() {
        let ctx = ParseContext {
            format: "markdown".to_owned(),
            parser_id: "default.markdown".to_owned(),
            origin_field: Some("body".to_owned()),
            content_type: Some("post".to_owned()),
        };
        let document = MarkdownParser
            .parse(
                ContentInput::Text(
                    "# Title\n\nHello world\n\n```rust\nfn main() {}\n```".to_owned(),
                ),
                &ctx,
            )
            .unwrap();

        assert_eq!(document.nodes.len(), 3);
        assert!(matches!(document.nodes[0], ContentNode::Heading(_)));
        assert!(matches!(document.nodes[1], ContentNode::Paragraph(_)));
        assert!(matches!(document.nodes[2], ContentNode::Block(_)));
    }
}
