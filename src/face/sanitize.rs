//! Output sanitization — §7.7.
//!
//! Model output is untrusted input that happens to originate inside the
//! process. It is truncated, stripped, and HTML-escaped before it can reach a
//! template. §20.7 drives this with control characters, ANSI escapes, 10 000
//! tokens, and `<script>` tags.

/// Hard ceiling on a rendered line, in characters. Well above any sane
/// `max_tokens`, and low enough that a runaway generation cannot fill a pane.
pub const MAX_COMMENTARY_CHARS: usize = 400;

/// Strip, truncate, and escape.
///
/// Returns `None` when nothing usable survives — which is a Face *failure*
/// (§7.8.3), not an empty success: a model producing nothing usable is failing
/// quietly, and the breaker needs to hear about it.
#[must_use]
pub fn sanitize(raw: &str) -> Option<String> {
    let stripped = strip_ansi(raw);

    let mut cleaned = String::with_capacity(stripped.len());
    let mut last_was_space = false;

    for ch in stripped.chars() {
        // Collapse all whitespace, including newlines, to single spaces: the
        // commentary pane is one line, not a document.
        if ch.is_whitespace() {
            if !cleaned.is_empty() && !last_was_space {
                cleaned.push(' ');
                last_was_space = true;
            }
            continue;
        }

        // Control and format characters never survive, including the
        // bidirectional overrides and isolates that can reorder rendered text.
        if ch.is_control()
            || matches!(ch, '\u{061C}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
        {
            continue;
        }

        cleaned.push(ch);
        last_was_space = false;
    }

    let truncated = truncate_on_char_boundary(cleaned.trim(), MAX_COMMENTARY_CHARS);
    if truncated.is_empty() {
        return None;
    }

    Some(escape_html(truncated))
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\u{1B}' {
            out.push(ch);
            continue;
        }
        // CSI sequences run until a byte in 0x40..=0x7E; anything else after
        // the escape is dropped along with it.
        if chars.peek() == Some(&'[') {
            chars.next();
            for terminator in chars.by_ref() {
                if ('\u{40}'..='\u{7E}').contains(&terminator) {
                    break;
                }
            }
        }
    }

    out
}

fn truncate_on_char_boundary(input: &str, max_chars: usize) -> &str {
    match input.char_indices().nth(max_chars) {
        Some((byte_index, _)) => &input[..byte_index],
        None => input,
    }
}

/// Escaped here rather than in the template, so that a template that forgets to
/// escape cannot reintroduce the hole.
fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_output_survives_intact() {
        assert_eq!(sanitize("Nice try."), Some("Nice try.".to_string()));
    }

    #[test]
    fn script_tags_cannot_reach_a_template() {
        let escaped = sanitize("<script>alert('x')</script>").expect("not empty");
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
        assert!(escaped.contains("&lt;script&gt;"));
    }

    #[test]
    fn ansi_escapes_are_stripped() {
        assert_eq!(sanitize("\u{1B}[31mred\u{1B}[0m"), Some("red".to_string()));
    }

    #[test]
    fn control_characters_and_bidi_overrides_are_stripped() {
        let cleaned = sanitize("a\u{0}b\u{202E}c\u{200B}d").expect("not empty");
        assert_eq!(cleaned, "abcd");
    }

    /// U+061C Arabic Letter Mark is a bidirectional formatting control like
    /// the override/isolate characters above, and must be stripped the same
    /// way — a model response can otherwise use it to reorder displayed text.
    #[test]
    fn arabic_letter_mark_is_stripped() {
        let cleaned = sanitize("a\u{061C}b").expect("not empty");
        assert_eq!(cleaned, "ab");
    }

    /// U+2066..U+2069 (bidi isolates) can reorder displayed text just as the
    /// override characters can, and must be stripped the same way.
    #[test]
    fn bidi_isolate_controls_are_stripped() {
        let cleaned = sanitize("a\u{2066}b\u{2069}c").expect("not empty");
        assert_eq!(cleaned, "abc");
    }

    #[test]
    fn newlines_collapse_to_single_spaces() {
        assert_eq!(
            sanitize("one\n\n\ttwo   three"),
            Some("one two three".to_string())
        );
    }

    #[test]
    fn ten_thousand_tokens_are_truncated() {
        let flood = "word ".repeat(10_000);
        let cleaned = sanitize(&flood).expect("not empty");
        assert!(cleaned.chars().count() <= MAX_COMMENTARY_CHARS);
    }

    #[test]
    fn truncation_never_splits_a_character() {
        let cleaned = sanitize(&"é".repeat(1_000)).expect("not empty");
        assert_eq!(cleaned.chars().count(), MAX_COMMENTARY_CHARS);
    }

    /// §7.8.3: empty after sanitization is a failure the breaker must see.
    #[test]
    fn nothing_usable_is_none_rather_than_an_empty_string() {
        assert_eq!(sanitize(""), None);
        assert_eq!(sanitize("   \n\t  "), None);
        assert_eq!(sanitize("\u{1B}[0m\u{0}\u{200B}"), None);
    }
}
