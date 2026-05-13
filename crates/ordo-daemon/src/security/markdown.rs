use reqwest::Url;

pub(crate) fn sanitize_markdown_links(markdown: &str) -> String {
    let mut output = String::with_capacity(markdown.len());
    let mut cursor = 0;
    while let Some(relative_start) = markdown[cursor..].find("](") {
        let link_start = cursor + relative_start;
        let destination_start = link_start + 2;
        let Some(relative_end) = markdown[destination_start..].find(')') else {
            break;
        };
        let destination_end = destination_start + relative_end;
        let destination = &markdown[destination_start..destination_end];
        output.push_str(&markdown[cursor..destination_start]);
        if is_safe_markdown_url(destination) {
            output.push_str(destination);
        } else {
            output.push_str("#unsafe-url-redacted");
        }
        output.push(')');
        cursor = destination_end + 1;
    }
    output.push_str(&markdown[cursor..]);
    output
}

pub(crate) fn is_safe_markdown_url(raw: &str) -> bool {
    let destination = raw.trim();
    if destination.is_empty()
        || destination.chars().any(char::is_control)
        || destination.contains('\\')
    {
        return false;
    }
    let url = destination
        .split_whitespace()
        .next()
        .unwrap_or(destination)
        .trim_matches(|character| character == '<' || character == '>');
    if url.is_empty() || url.starts_with("//") {
        return false;
    }
    if url.starts_with('#') {
        return true;
    }
    if has_absolute_scheme(url) {
        return Url::parse(url)
            .map(|parsed| parsed.scheme() == "https")
            .unwrap_or(false);
    }
    is_safe_relative_url(url)
}

fn has_absolute_scheme(value: &str) -> bool {
    let Some(colon_index) = value.find(':') else {
        return false;
    };
    let first_boundary = value
        .find(|character| matches!(character, '/' | '?' | '#'))
        .unwrap_or(value.len());
    colon_index < first_boundary
}

fn is_safe_relative_url(value: &str) -> bool {
    if value.starts_with('/') {
        return !value.starts_with("//") && !value.split('/').any(|segment| segment == "..");
    }
    !value.split('/').any(|segment| segment == "..")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_https_relative_and_local_anchor_links() {
        assert!(is_safe_markdown_url("https://example.com/path?q=1"));
        assert!(is_safe_markdown_url("docs/nested/page.md"));
        assert!(is_safe_markdown_url("/public/offers"));
        assert!(is_safe_markdown_url("#section"));
    }

    #[test]
    fn denies_script_data_http_file_and_malformed_urls() {
        for value in [
            "javascript:alert(1)",
            "data:text/html;base64,abc",
            "http://example.com",
            "file:///etc/passwd",
            "https://",
            "//example.com/path",
            "../secret.md",
            "docs/../../secret.md",
        ] {
            assert!(!is_safe_markdown_url(value), "{value}");
        }
    }

    #[test]
    fn sanitizes_markdown_link_and_image_destinations() {
        let markdown = "See [safe](https://example.com) and [bad](javascript:alert(1)) plus ![img](data:text/html;base64,abc).";

        let sanitized = sanitize_markdown_links(markdown);

        assert!(sanitized.contains("[safe](https://example.com)"));
        assert!(sanitized.contains("[bad](#unsafe-url-redacted)"));
        assert!(sanitized.contains("![img](#unsafe-url-redacted)"));
        assert!(!sanitized.contains("javascript:"));
        assert!(!sanitized.contains("data:text/html"));
    }

    #[test]
    fn preserves_long_nested_relative_markdown_image() {
        let nested = "media/generated/episode-01/frames/frame-0000123.png";
        let markdown = format!("![frame]({nested})");

        assert_eq!(sanitize_markdown_links(&markdown), markdown);
    }
}
