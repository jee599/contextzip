use scraper::{Html, Selector};

/// Check if input looks like HTML (contains DOCTYPE or <html tag)
pub fn is_html(input: &str) -> bool {
    let trimmed = input.trim_start();
    trimmed.starts_with("<!DOCTYPE")
        || trimmed.starts_with("<!doctype")
        || trimmed.starts_with("<html")
        || trimmed.starts_with("<HTML")
}

/// Extract main content from HTML, removing navigation noise.
/// Non-HTML input passes through unchanged.
pub fn extract_content(input: &str) -> String {
    if !is_html(input) {
        return input.to_string();
    }

    let document = Html::parse_document(input);

    // Collect text from the cleaned document
    // Strategy: remove unwanted elements, then extract text from what remains
    let mut output = String::new();

    // Try to find <main> or <article> first for focused extraction
    let main_sel = Selector::parse("main, article").expect("valid selector");
    let has_main = document.select(&main_sel).next().is_some();

    if has_main {
        for main_el in document.select(&main_sel) {
            extract_element_text(&main_el, &mut output);
        }
    } else {
        // Fall back to <body>, filtering out noise
        let body_sel = Selector::parse("body").expect("valid selector");
        if let Some(body) = document.select(&body_sel).next() {
            extract_element_text(&body, &mut output);
        } else {
            // No body tag — extract from root
            extract_element_text(&document.root_element(), &mut output);
        }
    }

    // Clean up whitespace: collapse multiple blank lines
    let lines: Vec<&str> = output.lines().collect();
    let mut result = Vec::new();
    let mut prev_blank = false;
    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_blank {
                result.push("");
            }
            prev_blank = true;
        } else {
            result.push(trimmed);
            prev_blank = false;
        }
    }

    // Trim leading/trailing blank lines
    let text = result.join("\n");
    text.trim().to_string()
}

/// Tags to completely skip (including all children)
fn should_skip_tag(tag: &str) -> bool {
    matches!(
        tag,
        "nav" | "header" | "footer" | "aside" | "script" | "style" | "noscript"
    )
}

/// Class/id patterns that indicate noise elements
const NOISE_PATTERNS: &[&str] = &[
    "cookie",
    "consent",
    "banner",
    "newsletter",
    "subscribe",
    "signup",
    "social",
    "share",
    "follow",
    "ad-",
    "ad_",
    "ads",
    "advert",
    "advertisement",
    "sponsor",
];

/// Check if an element's class or id contains any noise pattern
fn has_noise_class_or_id(element: &scraper::ElementRef) -> bool {
    let class_attr = element.value().attr("class").unwrap_or("");
    let id_attr = element.value().attr("id").unwrap_or("");

    let class_lower = class_attr.to_lowercase();
    let id_lower = id_attr.to_lowercase();

    for pattern in NOISE_PATTERNS {
        if class_lower.contains(pattern) || id_lower.contains(pattern) {
            return true;
        }
    }
    false
}

/// Recursively extract text from an element, skipping noise
fn extract_element_text(element: &scraper::element_ref::ElementRef, output: &mut String) {
    for child in element.children() {
        match child.value() {
            scraper::node::Node::Text(text) => {
                let t = text.text.trim();
                if !t.is_empty() {
                    output.push_str(t);
                    output.push('\n');
                }
            }
            scraper::node::Node::Element(el) => {
                let tag = el.name();

                // Skip noise tags entirely
                if should_skip_tag(tag) {
                    continue;
                }

                let child_ref = scraper::ElementRef::wrap(child);
                if let Some(ref child_el) = child_ref {
                    // Skip elements with noise class/id
                    if has_noise_class_or_id(child_el) {
                        continue;
                    }

                    // Handle special elements
                    match tag {
                        "img" => {
                            if let Some(alt) = el.attr("alt") {
                                let alt = alt.trim();
                                if !alt.is_empty() {
                                    output.push_str(&format!("[img: {}]", alt));
                                    output.push('\n');
                                }
                            }
                        }
                        "pre" | "code" => {
                            // Preserve code blocks with their content
                            let code_text = child_el.text().collect::<Vec<_>>().join("");
                            let trimmed = code_text.trim();
                            if !trimmed.is_empty() {
                                output.push_str("```\n");
                                output.push_str(trimmed);
                                output.push('\n');
                                output.push_str("```\n");
                            }
                        }
                        "table" => {
                            extract_table(child_el, output);
                        }
                        _ => {
                            // Block-level elements get a newline
                            let is_block = matches!(
                                tag,
                                "div"
                                    | "p"
                                    | "h1"
                                    | "h2"
                                    | "h3"
                                    | "h4"
                                    | "h5"
                                    | "h6"
                                    | "li"
                                    | "ul"
                                    | "ol"
                                    | "section"
                                    | "blockquote"
                                    | "figure"
                                    | "figcaption"
                                    | "details"
                                    | "summary"
                            );

                            extract_element_text(child_el, output);

                            if is_block {
                                output.push('\n');
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Extract table content in a readable format
fn extract_table(table: &scraper::ElementRef, output: &mut String) {
    let tr_sel = Selector::parse("tr").expect("valid selector");
    let th_sel = Selector::parse("th").expect("valid selector");
    let td_sel = Selector::parse("td").expect("valid selector");

    for row in table.select(&tr_sel) {
        let cells: Vec<String> = row
            .select(&th_sel)
            .chain(row.select(&td_sel))
            .map(|cell| cell.text().collect::<Vec<_>>().join("").trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if !cells.is_empty() {
            output.push_str(&cells.join(" | "));
            output.push('\n');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_nav_footer_keep_main() {
        let html = r#"<!DOCTYPE html>
<html>
<body>
    <nav><a href="/">Home</a><a href="/about">About</a></nav>
    <header><h1>Site Header</h1></header>
    <main>
        <h1>Main Content</h1>
        <p>This is the important article text.</p>
    </main>
    <footer>Copyright 2024</footer>
</body>
</html>"#;

        let result = extract_content(html);
        assert!(result.contains("Main Content"), "should keep main content");
        assert!(
            result.contains("important article text"),
            "should keep paragraph"
        );
        assert!(!result.contains("Home"), "should strip nav");
        assert!(!result.contains("About"), "should strip nav links");
        assert!(!result.contains("Site Header"), "should strip header");
        assert!(!result.contains("Copyright"), "should strip footer");
    }

    #[test]
    fn test_preserve_code_blocks() {
        let html = r#"<!DOCTYPE html>
<html>
<body>
<main>
    <p>Example code:</p>
    <pre><code>fn main() {
    println!("Hello, world!");
}</code></pre>
    <p>Inline <code>let x = 42;</code> works too.</p>
</main>
</body>
</html>"#;

        let result = extract_content(html);
        assert!(result.contains("```"), "should wrap code in backticks");
        assert!(
            result.contains("println!"),
            "should preserve code block content"
        );
        assert!(result.contains("let x = 42"), "should preserve inline code");
    }

    #[test]
    fn test_strip_cookie_banner_by_class() {
        let html = r#"<!DOCTYPE html>
<html>
<body>
    <div class="cookie-consent-banner">
        <p>We use cookies. Accept?</p>
        <button>Accept</button>
    </div>
    <main>
        <p>Real content here.</p>
    </main>
</body>
</html>"#;

        let result = extract_content(html);
        assert!(result.contains("Real content"), "should keep main content");
        assert!(
            !result.contains("We use cookies"),
            "should strip cookie banner"
        );
        assert!(!result.contains("Accept"), "should strip cookie button");
    }

    #[test]
    fn test_strip_script_style_tags() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <style>body { color: red; }</style>
    <script>console.log("tracking");</script>
</head>
<body>
    <main>
        <p>Visible text.</p>
    </main>
    <script>alert("popup");</script>
    <noscript>Enable JavaScript</noscript>
</body>
</html>"#;

        let result = extract_content(html);
        assert!(result.contains("Visible text"), "should keep main content");
        assert!(!result.contains("color: red"), "should strip style content");
        assert!(!result.contains("tracking"), "should strip script content");
        assert!(!result.contains("popup"), "should strip inline script");
        assert!(
            !result.contains("Enable JavaScript"),
            "should strip noscript"
        );
    }

    #[test]
    fn test_preserve_tables() {
        let html = r#"<!DOCTYPE html>
<html>
<body>
<main>
    <table>
        <tr><th>Name</th><th>Age</th></tr>
        <tr><td>Alice</td><td>30</td></tr>
        <tr><td>Bob</td><td>25</td></tr>
    </table>
</main>
</body>
</html>"#;

        let result = extract_content(html);
        assert!(result.contains("Name"), "should preserve table header");
        assert!(result.contains("Age"), "should preserve table header");
        assert!(result.contains("Alice"), "should preserve table data");
        assert!(result.contains("Bob"), "should preserve table data");
        assert!(result.contains("30"), "should preserve table data");
    }

    #[test]
    fn test_preserve_img_alt_text() {
        let html = r#"<!DOCTYPE html>
<html>
<body>
<main>
    <p>Check this out:</p>
    <img src="photo.jpg" alt="A beautiful sunset over the ocean">
    <img src="decorative.png">
</main>
</body>
</html>"#;

        let result = extract_content(html);
        assert!(
            result.contains("[img: A beautiful sunset over the ocean]"),
            "should preserve alt text"
        );
        // Decorative image with no alt should not produce output
        assert!(
            !result.contains("[img: ]"),
            "should not show empty alt text"
        );
    }

    #[test]
    fn test_non_html_passes_through() {
        let plain = "This is plain text output\nWith multiple lines\nNo HTML here";
        let result = extract_content(plain);
        assert_eq!(result, plain);
    }

    #[test]
    fn test_strip_ad_social_elements() {
        let html = r##"<!DOCTYPE html>
<html>
<body>
    <div id="advertisement-top">Buy now!</div>
    <div class="social-share-buttons">
        <a href="#">Share on Twitter</a>
        <a href="#">Share on Facebook</a>
    </div>
    <main>
        <p>Article content here.</p>
    </main>
    <div class="sponsor-block">Sponsored by Corp</div>
    <div class="newsletter-signup">
        <p>Subscribe to our newsletter</p>
    </div>
    <aside>Sidebar content</aside>
</body>
</html>"##;

        let result = extract_content(html);
        assert!(
            result.contains("Article content"),
            "should keep main content"
        );
        assert!(!result.contains("Buy now"), "should strip ad by id");
        assert!(
            !result.contains("Share on Twitter"),
            "should strip social by class"
        );
        assert!(
            !result.contains("Sponsored by"),
            "should strip sponsor by class"
        );
        assert!(
            !result.contains("Subscribe"),
            "should strip newsletter by class"
        );
        assert!(!result.contains("Sidebar"), "should strip aside element");
    }

    #[test]
    fn test_is_html_detection() {
        assert!(is_html("<!DOCTYPE html><html>"));
        assert!(is_html("<!doctype html><html>"));
        assert!(is_html("<html><body>Hi</body></html>"));
        assert!(is_html("<HTML><BODY>Hi</BODY></HTML>"));
        assert!(!is_html("Just plain text"));
        assert!(!is_html("{\"json\": true}"));
        assert!(!is_html(""));
    }

    #[test]
    fn test_article_tag_preserved() {
        let html = r#"<!DOCTYPE html>
<html>
<body>
    <nav><a href="/">Nav</a></nav>
    <article>
        <h2>Blog Post Title</h2>
        <p>Blog post body text.</p>
    </article>
    <footer>Footer</footer>
</body>
</html>"#;

        let result = extract_content(html);
        assert!(result.contains("Blog Post Title"));
        assert!(result.contains("Blog post body text"));
        assert!(!result.contains("Nav"));
        assert!(!result.contains("Footer"));
    }
}
