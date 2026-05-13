use anyhow::{anyhow, Result};
use markdown::mdast::Node;
use markdown::unist::Position;
use markdown::{to_mdast, Constructs, ParseOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::security::markdown::is_safe_markdown_url;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSpan {
    pub start_offset: usize,
    pub end_offset: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadingFact {
    pub depth: u8,
    pub text: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkFact {
    pub kind: String,
    pub label: String,
    pub protocol: String,
    pub destination_hash: String,
    pub safe: bool,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeBlockFact {
    pub language: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub value_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownArtifactFacts {
    pub headings: Vec<HeadingFact>,
    pub links: Vec<LinkFact>,
    pub code_blocks: Vec<CodeBlockFact>,
    pub front_matter: Option<SourceSpan>,
    pub replaceable_spans: Vec<SourceSpan>,
    pub diagnostics: Vec<MarkdownDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownQaSummary {
    pub heading_count: usize,
    pub link_count: usize,
    pub image_count: usize,
    pub code_block_count: usize,
    pub has_front_matter: bool,
    pub replaceable_span_count: usize,
    pub diagnostic_codes: Vec<String>,
}

pub fn inspect_markdown_artifact(_markdown: &str) -> Result<MarkdownArtifactFacts> {
    let markdown = _markdown;
    let ast = to_mdast(markdown, &ordo_parse_options()).map_err(|error| {
        anyhow!("parse markdown artifact without rendering or MDX execution: {error:?}")
    })?;
    let mut facts = MarkdownArtifactFacts {
        headings: Vec::new(),
        links: Vec::new(),
        code_blocks: Vec::new(),
        front_matter: None,
        replaceable_spans: Vec::new(),
        diagnostics: Vec::new(),
    };
    if markdown.trim().is_empty() {
        facts.diagnostics.push(MarkdownDiagnostic {
            code: "empty_document".to_string(),
            severity: "info".to_string(),
            message: "Markdown document is empty.".to_string(),
            span: None,
            value_hash: None,
        });
    }

    let mut heading_counts = BTreeMap::<String, usize>::new();
    collect_node_facts(&ast, &mut facts, &mut heading_counts);
    Ok(facts)
}

pub fn markdown_qa_summary(markdown: &str) -> Result<MarkdownQaSummary> {
    let facts = inspect_markdown_artifact(markdown)?;
    Ok(MarkdownQaSummary {
        heading_count: facts.headings.len(),
        link_count: facts
            .links
            .iter()
            .filter(|link| link.kind == "link")
            .count(),
        image_count: facts
            .links
            .iter()
            .filter(|link| link.kind == "image")
            .count(),
        code_block_count: facts.code_blocks.len(),
        has_front_matter: facts.front_matter.is_some(),
        replaceable_span_count: facts.replaceable_spans.len(),
        diagnostic_codes: facts
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.clone())
            .collect(),
    })
}

fn ordo_parse_options() -> ParseOptions {
    ParseOptions {
        constructs: Constructs {
            frontmatter: true,
            ..Constructs::gfm()
        },
        ..ParseOptions::gfm()
    }
}

fn collect_node_facts(
    node: &Node,
    facts: &mut MarkdownArtifactFacts,
    heading_counts: &mut BTreeMap<String, usize>,
) {
    match node {
        Node::Heading(heading) => {
            let text = heading
                .children
                .iter()
                .map(ToString::to_string)
                .collect::<String>()
                .trim()
                .to_string();
            if let Some(span) = span_from_position(heading.position.as_ref()) {
                let normalized = normalize_heading(&text);
                let count = heading_counts.entry(normalized).or_insert(0);
                *count += 1;
                if *count > 1 {
                    facts.diagnostics.push(MarkdownDiagnostic {
                        code: "duplicate_heading".to_string(),
                        severity: "warning".to_string(),
                        message: "Duplicate heading text detected.".to_string(),
                        span: Some(span.clone()),
                        value_hash: Some(stable_hash(&text)),
                    });
                }
                facts.headings.push(HeadingFact {
                    depth: heading.depth,
                    text,
                    span,
                });
            }
        }
        Node::Link(link) => {
            if let Some(span) = span_from_position(link.position.as_ref()) {
                let safe = is_safe_markdown_url(&link.url);
                let fact = LinkFact {
                    kind: "link".to_string(),
                    label: link
                        .children
                        .iter()
                        .map(ToString::to_string)
                        .collect::<String>(),
                    protocol: destination_protocol(&link.url),
                    destination_hash: stable_hash(&link.url),
                    safe,
                    span: span.clone(),
                };
                if !safe {
                    facts
                        .diagnostics
                        .push(unsafe_url_diagnostic(&span, &link.url));
                }
                facts.links.push(fact);
            }
        }
        Node::Image(image) => {
            if let Some(span) = span_from_position(image.position.as_ref()) {
                let safe = is_safe_markdown_url(&image.url);
                let fact = LinkFact {
                    kind: "image".to_string(),
                    label: image.alt.clone(),
                    protocol: destination_protocol(&image.url),
                    destination_hash: stable_hash(&image.url),
                    safe,
                    span: span.clone(),
                };
                if !safe {
                    facts
                        .diagnostics
                        .push(unsafe_url_diagnostic(&span, &image.url));
                }
                facts.links.push(fact);
            }
        }
        Node::Definition(definition) => {
            if let Some(span) = span_from_position(definition.position.as_ref()) {
                if !is_safe_markdown_url(&definition.url) {
                    facts
                        .diagnostics
                        .push(unsafe_url_diagnostic(&span, &definition.url));
                }
            }
        }
        Node::Code(code) => {
            if let Some(span) = span_from_position(code.position.as_ref()) {
                facts.code_blocks.push(CodeBlockFact {
                    language: code
                        .lang
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(|value| value.to_ascii_lowercase()),
                    span,
                });
            }
        }
        Node::Yaml(yaml) => {
            if facts.front_matter.is_none() {
                facts.front_matter = span_from_position(yaml.position.as_ref());
            }
        }
        Node::Toml(toml) => {
            if facts.front_matter.is_none() {
                facts.front_matter = span_from_position(toml.position.as_ref());
            }
        }
        Node::Text(text) => {
            if let Some(span) = span_from_position(text.position.as_ref()) {
                facts.replaceable_spans.push(span);
            }
        }
        Node::Html(html) => {
            facts.diagnostics.push(MarkdownDiagnostic {
                code: "html_not_executed".to_string(),
                severity: "warning".to_string(),
                message: "Raw HTML was parsed as inert markdown content and was not executed."
                    .to_string(),
                span: span_from_position(html.position.as_ref()),
                value_hash: Some(stable_hash(&html.value)),
            });
        }
        Node::MdxjsEsm(mdx) => {
            facts.diagnostics.push(mdx_not_executed_diagnostic(
                span_from_position(mdx.position.as_ref()),
                &mdx.value,
            ));
        }
        Node::MdxFlowExpression(mdx) => {
            facts.diagnostics.push(mdx_not_executed_diagnostic(
                span_from_position(mdx.position.as_ref()),
                &mdx.value,
            ));
        }
        Node::MdxTextExpression(mdx) => {
            facts.diagnostics.push(mdx_not_executed_diagnostic(
                span_from_position(mdx.position.as_ref()),
                &mdx.value,
            ));
        }
        Node::MdxJsxFlowElement(mdx) => {
            facts.diagnostics.push(MarkdownDiagnostic {
                code: "mdx_not_executed".to_string(),
                severity: "warning".to_string(),
                message: "MDX JSX was not executed.".to_string(),
                span: span_from_position(mdx.position.as_ref()),
                value_hash: None,
            });
        }
        Node::MdxJsxTextElement(mdx) => {
            facts.diagnostics.push(MarkdownDiagnostic {
                code: "mdx_not_executed".to_string(),
                severity: "warning".to_string(),
                message: "MDX JSX was not executed.".to_string(),
                span: span_from_position(mdx.position.as_ref()),
                value_hash: None,
            });
        }
        _ => {}
    }

    if !matches!(
        node,
        Node::InlineCode(_) | Node::Code(_) | Node::Yaml(_) | Node::Toml(_)
    ) {
        if let Some(children) = node.children() {
            for child in children {
                collect_node_facts(child, facts, heading_counts);
            }
        }
    }
}

fn unsafe_url_diagnostic(span: &SourceSpan, destination: &str) -> MarkdownDiagnostic {
    MarkdownDiagnostic {
        code: "unsafe_url".to_string(),
        severity: "error".to_string(),
        message: "Unsafe markdown URL was rejected for trusted rendering or mutation.".to_string(),
        span: Some(span.clone()),
        value_hash: Some(stable_hash(destination)),
    }
}

fn mdx_not_executed_diagnostic(span: Option<SourceSpan>, value: &str) -> MarkdownDiagnostic {
    MarkdownDiagnostic {
        code: "mdx_not_executed".to_string(),
        severity: "warning".to_string(),
        message: "MDX content was parsed as inert markdown content and was not executed."
            .to_string(),
        span,
        value_hash: Some(stable_hash(value)),
    }
}

fn span_from_position(position: Option<&Position>) -> Option<SourceSpan> {
    position.map(|position| SourceSpan {
        start_offset: position.start.offset,
        end_offset: position.end.offset,
        start_line: position.start.line,
        start_column: position.start.column,
        end_line: position.end.line,
        end_column: position.end.column,
    })
}

fn destination_protocol(destination: &str) -> String {
    let trimmed = destination.trim();
    if trimmed.starts_with('#') {
        return "anchor".to_string();
    }
    if trimmed.starts_with('/') {
        return "absolute_path".to_string();
    }
    if let Some(colon_index) = trimmed.find(':') {
        let boundary = trimmed
            .find(|character| matches!(character, '/' | '?' | '#'))
            .unwrap_or(trimmed.len());
        if colon_index < boundary {
            return trimmed[..colon_index].to_ascii_lowercase();
        }
    }
    "relative".to_string()
}

fn normalize_heading(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn stable_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("sha256:{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_headings_links_code_front_matter_and_summary() {
        let markdown = concat!(
            "---\n",
            "title: Pilot Brief\n",
            "---\n\n",
            "# Pilot Brief\n\n",
            "Review the [offer](https://example.com/offers) and ![frame](media/frame.png).\n\n",
            "## Next Steps\n\n",
            "```rust\n",
            "let value = \"{{do_not_replace}}\";\n",
            "```\n\n",
            "Replace {{pilot_city}} in prose.\n"
        );

        let facts = inspect_markdown_artifact(markdown).unwrap();

        assert_eq!(
            facts
                .headings
                .iter()
                .map(|heading| (heading.depth, heading.text.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "Pilot Brief"), (2, "Next Steps")]
        );
        assert!(facts.front_matter.is_some());
        assert_eq!(facts.code_blocks.len(), 1);
        assert_eq!(facts.code_blocks[0].language.as_deref(), Some("rust"));
        assert_eq!(facts.links.len(), 2);
        assert!(facts
            .links
            .iter()
            .any(|link| link.kind == "link" && link.protocol == "https" && link.safe));
        assert!(facts
            .links
            .iter()
            .any(|link| link.kind == "image" && link.protocol == "relative" && link.safe));

        let replaceable_text = facts
            .replaceable_spans
            .iter()
            .map(|span| &markdown[span.start_offset..span.end_offset])
            .collect::<Vec<_>>()
            .join("\n");
        assert!(replaceable_text.contains("{{pilot_city}}"));
        assert!(!replaceable_text.contains("{{do_not_replace}}"));

        let summary = markdown_qa_summary(markdown).unwrap();
        assert_eq!(summary.heading_count, 2);
        assert_eq!(summary.link_count, 1);
        assert_eq!(summary.image_count, 1);
        assert_eq!(summary.code_block_count, 1);
        assert!(summary.has_front_matter);
        assert_eq!(summary.diagnostic_codes, Vec::<String>::new());
    }

    #[test]
    fn reports_unsafe_urls_and_html_without_echoing_secret_values() {
        let markdown = concat!(
            "# Links\n\n",
            "Bad [link](javascript:alert('sk_live_secret_123')) and ",
            "![pixel](data:text/html;base64,sk_live_secret_456).\n\n",
            "<script>sk_live_secret_789</script>\n"
        );

        let facts = inspect_markdown_artifact(markdown).unwrap();

        assert_eq!(facts.links.len(), 2);
        assert!(facts.links.iter().all(|link| !link.safe));
        assert!(facts
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "unsafe_url"));
        assert!(facts
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "html_not_executed"));
        let diagnostics = serde_json::to_string(&facts.diagnostics).unwrap();
        assert!(!diagnostics.contains("sk_live_secret"));
        assert!(!diagnostics.contains("javascript:alert"));
        assert!(!diagnostics.contains("data:text/html"));
    }

    #[test]
    fn handles_empty_and_malformed_markdown_deterministically() {
        let empty = inspect_markdown_artifact("").unwrap();
        assert!(empty.headings.is_empty());
        assert!(empty
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "empty_document"));

        let malformed = inspect_markdown_artifact("# Open [link\n\n- item\n  - nested").unwrap();
        assert_eq!(malformed.headings.len(), 1);
        assert!(malformed.diagnostics.is_empty());
    }

    #[test]
    fn ignores_front_matter_and_variables_inside_code_or_inline_code() {
        let markdown = concat!(
            "# Contract\n\n",
            "```markdown\n",
            "---\n",
            "title: Not Front Matter\n",
            "---\n",
            "{{inside_fence}}\n",
            "```\n\n",
            "Inline `{{inside_inline}}` and prose {{outside}}.\n"
        );

        let facts = inspect_markdown_artifact(markdown).unwrap();

        assert!(facts.front_matter.is_none());
        let replaceable_text = facts
            .replaceable_spans
            .iter()
            .map(|span| &markdown[span.start_offset..span.end_offset])
            .collect::<Vec<_>>()
            .join("\n");
        assert!(replaceable_text.contains("{{outside}}"));
        assert!(!replaceable_text.contains("{{inside_fence}}"));
        assert!(!replaceable_text.contains("{{inside_inline}}"));
    }

    #[test]
    fn reports_duplicate_headings_without_private_payloads() {
        let markdown = "# Roadmap sk_live_secret\n\n## Step\n\n## Step\n";

        let facts = inspect_markdown_artifact(markdown).unwrap();

        assert_eq!(facts.headings.len(), 3);
        assert!(facts
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "duplicate_heading"));
        let diagnostics = serde_json::to_string(&facts.diagnostics).unwrap();
        assert!(!diagnostics.contains("sk_live_secret"));
        assert!(!diagnostics.contains("Roadmap"));
    }
}
