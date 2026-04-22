use ammonia::clean;
use cycms_core::{Error, Result};
use cycms_host_types::{
    AssetGraph, ContentDocument, ContentNode, HeadNode, HtmlNode, PageDocument, PageNode,
    RenderedPage,
};

pub trait HtmlRenderer {
    fn render(&self, page: &PageDocument, assets: &AssetGraph) -> Result<RenderedPage>;
}

pub struct DefaultHtmlRenderer;

impl HtmlRenderer for DefaultHtmlRenderer {
    fn render(&self, page: &PageDocument, assets: &AssetGraph) -> Result<RenderedPage> {
        if page.route_id.trim().is_empty() {
            return Err(Error::ValidationError {
                message: "page route_id must not be empty".to_owned(),
                details: None,
            });
        }

        let mut html = String::from("<!doctype html><html><head>");
        html.push_str(&render_head(&page.head, assets));
        html.push_str("</head><body>");

        for node in &page.body {
            html.push_str(&render_page_node(node)?);
        }

        for action in &page.actions {
            html.push_str(&format!(
                "<form data-action-id=\"{}\" method=\"{}\" action=\"{}\"><button type=\"submit\">{}</button></form>",
                escape_html(&action.id),
                escape_html(&action.method),
                escape_html(&action.href),
                escape_html(&action.label),
            ));
        }

        html.push_str(&render_body_assets(assets)?);
        html.push_str("</body></html>");

        Ok(RenderedPage {
            status: page.status,
            html,
            assets: assets.clone(),
        })
    }
}

fn render_head(head: &[HeadNode], assets: &AssetGraph) -> String {
    let mut out = String::new();
    for node in head {
        match node {
            HeadNode::Title { text } => {
                out.push_str(&format!("<title>{}</title>", escape_html(text)));
            }
            HeadNode::Meta { name, content } => {
                out.push_str(&format!(
                    "<meta name=\"{}\" content=\"{}\">",
                    escape_html(name),
                    escape_html(content),
                ));
            }
            HeadNode::Link { rel, href } => {
                out.push_str(&format!(
                    "<link rel=\"{}\" href=\"{}\">",
                    escape_html(rel),
                    escape_html(href),
                ));
            }
        }
    }

    for style in &assets.styles {
        out.push_str(&format!(
            "<link rel=\"stylesheet\" data-asset-id=\"{}\" href=\"{}\">",
            escape_html(&style.id),
            escape_html(&style.href),
        ));
    }

    out
}

fn render_body_assets(assets: &AssetGraph) -> Result<String> {
    let mut out = String::new();

    for data in &assets.inline_data {
        let payload = serde_json::to_string(&data.value).map_err(|source| Error::Internal {
            message: format!("serialize inline data asset {}: {source}", data.id),
            source: None,
        })?;
        out.push_str(&format!(
            "<script type=\"application/json\" id=\"{}\">{}</script>",
            escape_html(&data.id),
            escape_html(&payload),
        ));
    }

    for script in &assets.scripts {
        out.push_str(&format!(
            "<script data-asset-id=\"{}\" src=\"{}\"></script>",
            escape_html(&script.id),
            escape_html(&script.href),
        ));
    }

    for module in &assets.modules {
        out.push_str(&format!(
            "<script type=\"module\" data-asset-id=\"{}\" src=\"{}\"></script>",
            escape_html(&module.id),
            escape_html(&module.href),
        ));
    }

    for boot in &assets.island_boot {
        let payload = serde_json::to_string(&boot.props).map_err(|source| Error::Internal {
            message: format!("serialize island boot payload {}: {source}", boot.island_id),
            source: None,
        })?;
        out.push_str(&format!(
                "<script type=\"application/json\" data-island-boot=\"{}\" data-module=\"{}\">{}</script>",
                escape_html(&boot.island_id),
                escape_html(&boot.module),
                escape_html(&payload),
            ));
    }

    Ok(out)
}

fn render_page_node(node: &PageNode) -> Result<String> {
    Ok(match node {
        PageNode::Layout(layout) => format!(
            "<div data-layout=\"{}\">{}</div>",
            escape_html(&layout.name),
            render_page_nodes(&layout.children)?,
        ),
        PageNode::Region(region) => format!(
            "<section data-region=\"{}\">{}</section>",
            escape_html(&region.name),
            render_page_nodes(&region.children)?,
        ),
        PageNode::Fragment(fragment) => return render_page_nodes(&fragment.children),
        PageNode::Html(HtmlNode {
            tag,
            attributes,
            children,
        }) => {
            validate_tag_name(tag)?;
            let attrs = attributes
                .iter()
                .map(|(key, value)| {
                    validate_attribute_name(key)?;
                    Ok::<String, Error>(format!(" {}=\"{}\"", key, escape_html(value)))
                })
                .collect::<Result<String>>()?;
            format!("<{tag}{attrs}>{}</{tag}>", render_page_nodes(children)?)
        }
        PageNode::Text(text) => escape_html(&text.value),
        PageNode::SlotOutlet(slot) => format!(
            "<div data-slot-outlet=\"{}\">{}</div>",
            escape_html(&slot.slot),
            render_page_nodes(&slot.fallback)?,
        ),
        PageNode::ContentOutlet(outlet) => return render_content_document(&outlet.content),
        PageNode::Island(island) => {
            let props = serde_json::to_string(&island.props).map_err(|source| Error::Internal {
                message: format!("serialize island props {}: {source}", island.id),
                source: None,
            })?;
            format!(
                "<div data-island-id=\"{}\" data-island-component=\"{}\" data-island-props=\"{}\"></div>",
                escape_html(&island.id),
                escape_html(&island.component),
                escape_html(&props),
            )
        }
    })
}

fn render_page_nodes(nodes: &[PageNode]) -> Result<String> {
    let mut out = String::new();
    for node in nodes {
        out.push_str(&render_page_node(node)?);
    }
    Ok(out)
}

fn render_content_document(document: &ContentDocument) -> Result<String> {
    let mut out = String::new();
    for node in &document.nodes {
        out.push_str(&render_content_node(node)?);
    }
    Ok(out)
}

fn render_content_node(node: &ContentNode) -> Result<String> {
    Ok(match node {
        ContentNode::Heading(heading) => {
            let level = heading.level.clamp(1, 6);
            format!("<h{}>{}</h{}>", level, escape_html(&heading.text), level,)
        }
        ContentNode::Paragraph(paragraph) => format!("<p>{}</p>", escape_html(&paragraph.text)),
        ContentNode::Block(block) => format!(
            "<div data-block-kind=\"{}\">{}</div>",
            escape_html(&block.kind),
            escape_html(
                &serde_json::to_string(&block.data).map_err(|source| Error::Internal {
                    message: format!("serialize content block {}: {source}", block.kind),
                    source: None,
                })?
            ),
        ),
        ContentNode::SlotCall(slot) => {
            format!("<div data-slot-call=\"{}\"></div>", escape_html(&slot.slot),)
        }
        ContentNode::Embed(embed) => format!(
            "<figure data-embed-url=\"{}\"><a href=\"{}\">{}</a></figure>",
            escape_html(&embed.url),
            escape_html(&embed.url),
            escape_html(embed.title.as_deref().unwrap_or(&embed.url)),
        ),
        ContentNode::RawHtml(html) => clean(html),
    })
}

fn validate_tag_name(tag: &str) -> Result<()> {
    let valid = !tag.is_empty()
        && tag
            .chars()
            .enumerate()
            .all(|(index, ch)| match (index, ch) {
                (0, c) => c.is_ascii_alphabetic(),
                (_, c) => c.is_ascii_alphanumeric() || c == '-' || c == ':',
            });
    if !valid {
        return Err(Error::ValidationError {
            message: format!("invalid html tag name {tag:?}"),
            details: None,
        });
    }
    Ok(())
}

fn validate_attribute_name(name: &str) -> Result<()> {
    let valid = !name.is_empty()
        && !name.starts_with("on")
        && name
            .chars()
            .enumerate()
            .all(|(index, ch)| match (index, ch) {
                (0, c) => c.is_ascii_alphabetic() || c == ':' || c == '_',
                (_, c) => c.is_ascii_alphanumeric() || matches!(c, '-' | ':' | '_' | '.'),
            });
    if !valid {
        return Err(Error::ValidationError {
            message: format!("invalid html attribute name {name:?}"),
            details: None,
        });
    }
    Ok(())
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use cycms_host_types::{
        AssetGraph, AssetReference, ContentDocument, ContentNode, ContentOutletNode,
        ContentSourceMeta, HeadNode, HeadingNode, IslandBootSpec, IslandMount, PageAction,
        PageDocument, PageNode, TextNode,
    };
    use http::StatusCode;
    use serde_json::json;

    use super::*;

    #[test]
    fn renders_page_document_with_assets_and_content() {
        let page = PageDocument {
            route_id: "post.show".to_owned(),
            status: StatusCode::OK,
            head: vec![HeadNode::Title {
                text: "Hello".to_owned(),
            }],
            body: vec![
                PageNode::ContentOutlet(ContentOutletNode {
                    content: ContentDocument {
                        schema_version: 1,
                        source: ContentSourceMeta {
                            format: "markdown".to_owned(),
                            parser_id: "default.markdown".to_owned(),
                            origin_field: Some("body".to_owned()),
                            content_type: Some("post".to_owned()),
                        },
                        nodes: vec![ContentNode::Heading(HeadingNode {
                            level: 1,
                            text: "Hello".to_owned(),
                        })],
                        diagnostics: Vec::new(),
                    },
                }),
                PageNode::Text(TextNode {
                    value: "Footer".to_owned(),
                }),
                PageNode::Island(IslandMount {
                    id: "editor".to_owned(),
                    component: "EditorIsland".to_owned(),
                    props: json!({ "entryId": "post-1" }),
                    module_url: None,
                }),
            ],
            actions: vec![PageAction {
                id: "publish".to_owned(),
                label: "Publish".to_owned(),
                method: "post".to_owned(),
                href: "/actions/publish".to_owned(),
            }],
            islands: Vec::new(),
            cache_tags: vec!["post:1".to_owned()],
        };
        let assets = AssetGraph {
            styles: vec![AssetReference {
                id: "theme".to_owned(),
                href: "/assets/theme.css".to_owned(),
            }],
            modules: vec![AssetReference {
                id: "islands".to_owned(),
                href: "/assets/islands.js".to_owned(),
            }],
            island_boot: vec![IslandBootSpec {
                island_id: "editor".to_owned(),
                module: "/assets/islands.js".to_owned(),
                props: json!({ "entryId": "post-1" }),
            }],
            ..AssetGraph::default()
        };

        let rendered = DefaultHtmlRenderer.render(&page, &assets).unwrap();

        assert!(rendered.html.contains("<title>Hello</title>"));
        assert!(rendered.html.contains("<h1>Hello</h1>"));
        assert!(rendered.html.contains("/assets/theme.css"));
        assert!(rendered.html.contains("data-island-id=\"editor\""));
        assert!(rendered.html.contains("/actions/publish"));
    }

    #[test]
    fn sanitizes_raw_html_content() {
        let page = PageDocument {
            route_id: "raw-html".to_owned(),
            status: StatusCode::OK,
            head: Vec::new(),
            body: vec![PageNode::ContentOutlet(ContentOutletNode {
                content: ContentDocument {
                    schema_version: 1,
                    source: ContentSourceMeta {
                        format: "html".to_owned(),
                        parser_id: "default.html".to_owned(),
                        origin_field: Some("body".to_owned()),
                        content_type: Some("page".to_owned()),
                    },
                    nodes: vec![ContentNode::RawHtml(
                        "<script>alert(1)</script><p>safe</p>".to_owned(),
                    )],
                    diagnostics: Vec::new(),
                },
            })],
            actions: Vec::new(),
            islands: Vec::new(),
            cache_tags: Vec::new(),
        };

        let rendered = DefaultHtmlRenderer
            .render(&page, &AssetGraph::default())
            .unwrap();
        assert!(!rendered.html.contains("<script>"));
        assert!(rendered.html.contains("<p>safe</p>"));
    }

    #[test]
    fn rejects_invalid_html_node_tags() {
        let page = PageDocument {
            route_id: "invalid-tag".to_owned(),
            status: StatusCode::OK,
            head: Vec::new(),
            body: vec![PageNode::Html(HtmlNode {
                tag: "img onerror=alert(1)".to_owned(),
                attributes: Default::default(),
                children: Vec::new(),
            })],
            actions: Vec::new(),
            islands: Vec::new(),
            cache_tags: Vec::new(),
        };

        assert!(
            DefaultHtmlRenderer
                .render(&page, &AssetGraph::default())
                .is_err()
        );
    }
}
