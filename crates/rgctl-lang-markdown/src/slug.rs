//! GitHub-style heading slugs for markdown qualified names.

/// Convert heading text to an ASCII slug (lowercase, hyphenated).
pub fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_hyphen = false;
    for ch in text.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            prev_hyphen = false;
        } else if lower.is_ascii_whitespace() || lower == '-' || lower == '_' {
            if !out.is_empty() && !prev_hyphen {
                out.push('-');
                prev_hyphen = true;
            }
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "heading".to_string()
    } else {
        out
    }
}

/// Assign a unique slug within one file, appending `-2`, `-3`, … on collision.
pub fn unique_slug(base: &str, used: &mut std::collections::HashMap<String, usize>) -> String {
    let count = used.entry(base.to_string()).or_insert(0);
    *count += 1;
    if *count == 1 {
        base.to_string()
    } else {
        format!("{base}-{count}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn slugify_checkout_flow() {
        assert_eq!(slugify("Checkout Flow"), "checkout-flow");
    }

    #[test]
    fn slugify_empty_punctuation_becomes_heading() {
        assert_eq!(slugify("!!!"), "heading");
    }

    #[test]
    fn unique_slug_suffixes_duplicates() {
        let mut used = HashMap::new();
        assert_eq!(unique_slug("overview", &mut used), "overview");
        assert_eq!(unique_slug("overview", &mut used), "overview-2");
        assert_eq!(unique_slug("overview", &mut used), "overview-3");
    }
}
