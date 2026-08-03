fn sanitize_catalog_chunk(value: &str) -> Option<String> {
    if value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return None;
    }
    let mut text = value.replace("\r\n", "\n").replace('\r', "\n");
    for tag in ["script", "style", "noscript"] {
        text = strip_html_element_blocks(&text, tag);
    }
    text = strip_markdown_link_targets(&text);
    text = strip_catalog_html_tags(&text);
    let lines = text
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let text = lines.join(" ");
    let text = text.trim();
    (!text.is_empty() && text.chars().count() <= SOURCE_CATALOG_MAX_CHUNK_CHARS)
        .then(|| text.to_string())
}

/// Keep visible Markdown labels while removing transport URLs and image
/// syntax. The source anchor remains available in the Host-owned source
/// ledger, so inline targets add prompt weight without adding evidence.
fn strip_markdown_link_targets(value: &str) -> String {
    let without_images = strip_markdown_targets(value, true);
    let without_links = strip_markdown_targets(&without_images, false);
    strip_orphan_markdown_targets(&without_links)
}

fn strip_markdown_targets(value: &str, images_only: bool) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0usize;
    while cursor < characters.len() {
        let image = characters[cursor] == '!'
            && characters
                .get(cursor + 1)
                .is_some_and(|character| *character == '[');
        if images_only != image {
            output.push(characters[cursor]);
            cursor += 1;
            continue;
        }
        let label_start = if image {
            cursor + 2
        } else if !images_only && characters[cursor] == '[' {
            cursor + 1
        } else {
            output.push(characters[cursor]);
            cursor += 1;
            continue;
        };
        let Some(label_end) = characters[label_start..]
            .iter()
            .position(|character| *character == ']')
            .map(|offset| label_start + offset)
        else {
            output.push(characters[cursor]);
            cursor += 1;
            continue;
        };
        if characters.get(label_end + 1) != Some(&'(') {
            output.push(characters[cursor]);
            cursor += 1;
            continue;
        }
        let mut target_end = label_end + 2;
        let mut depth = 1usize;
        while target_end < characters.len() && depth > 0 {
            match characters[target_end] {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            target_end += 1;
        }
        if depth != 0 {
            output.push(characters[cursor]);
            cursor += 1;
            continue;
        }
        let label = characters[label_start..label_end]
            .iter()
            .collect::<String>();
        if !label.trim().is_empty() {
            output.push_str(label.trim());
        }
        output.push(' ');
        cursor = target_end;
    }
    output
}

fn strip_orphan_markdown_targets(value: &str) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0usize;
    while cursor < characters.len() {
        if characters[cursor] != ']'
            || characters.get(cursor + 1).is_none_or(|character| *character != '(')
        {
            output.push(characters[cursor]);
            cursor += 1;
            continue;
        }
        let mut target_end = cursor + 2;
        let mut depth = 1usize;
        while target_end < characters.len() && depth > 0 {
            match characters[target_end] {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            target_end += 1;
        }
        if depth == 0 {
            output.push(' ');
            cursor = target_end;
        } else {
            output.push(characters[cursor]);
            cursor += 1;
        }
    }
    output
}

fn strip_catalog_html_tags(value: &str) -> String {
    static HTML_TAG: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let pattern = HTML_TAG.get_or_init(|| {
        regex::Regex::new(r"(?is)\\?</?[a-z][^>]{0,1200}>")
            .expect("static HTML tag regex")
    });
    pattern.replace_all(value, " ").into_owned()
}

fn strip_html_element_blocks(value: &str, tag: &str) -> String {
    let mut output = value.to_string();
    let opening = format!("<{tag}");
    let closing = format!("</{tag}>");
    loop {
        let lower = output.to_ascii_lowercase();
        let Some(start) = lower.find(&opening) else {
            break;
        };
        let end = lower[start..]
            .find(&closing)
            .map(|offset| start + offset + closing.len())
            .or_else(|| lower[start..].find('>').map(|offset| start + offset + 1))
            .unwrap_or(output.len());
        output.replace_range(start..end, " ");
    }
    output
}
