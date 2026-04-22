use std::collections::BTreeMap;

use cycms_core::Result;
use cycms_host_types::{
    ContentDocument, ContentOutletNode, HeadNode, IslandMount, LayoutNode,
    PAGE_DOCUMENT_SCHEMA_VERSION, PageAction, PageDocument, PageNode, RegionNode,
};
use http::StatusCode;

#[derive(Debug, Clone, PartialEq)]
pub struct PageBuildInput {
    pub route_id: String,
    pub status: StatusCode,
    pub head: Vec<HeadNode>,
    pub content: Option<ContentDocument>,
    pub body: Vec<PageNode>,
    pub actions: Vec<PageAction>,
    pub islands: Vec<IslandMount>,
    pub cache_tags: Vec<String>,
    pub layout_name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PageBuildContext {
    pub regions: BTreeMap<String, Vec<PageNode>>,
}

pub trait PageBuilder {
    fn build(&self, input: PageBuildInput, ctx: &PageBuildContext) -> Result<PageDocument>;
}

pub struct DefaultPageBuilder;

impl PageBuilder for DefaultPageBuilder {
    fn build(&self, input: PageBuildInput, ctx: &PageBuildContext) -> Result<PageDocument> {
        let mut body = input.body;

        if let Some(content) = input.content {
            body.insert(0, PageNode::ContentOutlet(ContentOutletNode { content }));
        }

        for (region_name, nodes) in &ctx.regions {
            body.push(PageNode::Region(RegionNode {
                name: region_name.clone(),
                children: nodes.clone(),
            }));
        }

        if let Some(layout_name) = input.layout_name {
            body = vec![PageNode::Layout(LayoutNode {
                name: layout_name,
                children: body,
            })];
        }

        Ok(PageDocument {
            route_id: format!("v{PAGE_DOCUMENT_SCHEMA_VERSION}:{}", input.route_id),
            status: input.status,
            head: input.head,
            body,
            actions: input.actions,
            islands: input.islands,
            cache_tags: input.cache_tags,
        })
    }
}
