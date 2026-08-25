//! 256-bit token bloom sketches for eager structural indexing at extract time.

use std::collections::HashSet;

/// Number of bits in each function token sketch.
pub const TOKEN_BLOOM_BITS: usize = 256;

/// Number of u64 words backing a sketch (`TOKEN_BLOOM_BITS / 64`).
pub const TOKEN_BLOOM_WORDS: usize = 4;

/// Hash probes per token when inserting or querying the bloom filter.
const PROBES_PER_TOKEN: u8 = 2;

/// Minimum token length retained after normalization.
pub const MIN_TOKEN_LEN: usize = 3;

/// 256-bit bloom filter packed as four little-endian u64 words.
pub type TokenBloom = [u64; TOKEN_BLOOM_WORDS];

/// Build a token bloom from declaration metadata and optional body text.
pub fn build_token_bloom(
    name: &str,
    qualified_name: Option<&str>,
    signature: Option<&str>,
    body: Option<&str>,
) -> TokenBloom {
    let mut bloom = empty_bloom();
    tokenize_into_bloom(name, &mut bloom);
    if let Some(qn) = qualified_name {
        tokenize_into_bloom(qn, &mut bloom);
    }
    if let Some(sig) = signature {
        tokenize_into_bloom(sig, &mut bloom);
    }
    if let Some(text) = body {
        tokenize_into_bloom(text, &mut bloom);
    }
    bloom
}

/// Empty bloom (all bits clear).
pub fn empty_bloom() -> TokenBloom {
    [0; TOKEN_BLOOM_WORDS]
}

/// Insert one normalized token into the bloom filter.
pub fn insert_token(bloom: &mut TokenBloom, token: &str) {
    for probe in 0..PROBES_PER_TOKEN {
        set_bit(bloom, probe_index(token, probe));
    }
}

/// True when at least one probe for `keyword` is present in the bloom.
pub fn keyword_in_bloom(keyword: &str, bloom: &TokenBloom) -> bool {
    (0..PROBES_PER_TOKEN).any(|probe| test_bit(bloom, probe_index(keyword, probe)))
}

/// True when every keyword matches via independent bloom probes (AND semantics).
pub fn satisfies_keyword_and(keywords: &[String], bloom: &TokenBloom) -> bool {
    if keywords.is_empty() {
        return true;
    }
    keywords
        .iter()
        .all(|keyword| keyword_in_bloom(keyword, bloom))
}

/// Fraction of query keywords matched in the bloom sketch.
pub fn keyword_overlap_score(keywords: &[String], bloom: &TokenBloom) -> f64 {
    if keywords.is_empty() {
        return 0.0;
    }
    let matched = keywords
        .iter()
        .filter(|keyword| keyword_in_bloom(keyword, bloom))
        .count();
    matched as f64 / keywords.len() as f64
}

/// Split on camelCase, snake_case, and non-alphanumeric boundaries into a set.
pub fn tokenize_string_into(text: &str, set: &mut HashSet<String>) {
    let mut current_token = String::with_capacity(16);

    for c in text.chars() {
        if c.is_alphanumeric() {
            if !current_token.is_empty()
                && c.is_uppercase()
                && current_token
                    .chars()
                    .last()
                    .is_some_and(|last| last.is_lowercase())
            {
                push_token_set(&current_token, set);
                current_token.clear();
            }
            current_token.push(c);
        } else {
            push_token_set(&current_token, set);
            current_token.clear();
        }
    }
    push_token_set(&current_token, set);
}

fn tokenize_into_bloom(text: &str, bloom: &mut TokenBloom) {
    let mut current_token = String::with_capacity(16);

    for c in text.chars() {
        if c.is_alphanumeric() {
            if !current_token.is_empty()
                && c.is_uppercase()
                && current_token
                    .chars()
                    .last()
                    .is_some_and(|last| last.is_lowercase())
            {
                push_token_bloom(&current_token, bloom);
                current_token.clear();
            }
            current_token.push(c);
        } else {
            push_token_bloom(&current_token, bloom);
            current_token.clear();
        }
    }
    push_token_bloom(&current_token, bloom);
}

fn push_token_set(token: &str, set: &mut HashSet<String>) {
    if token.len() >= MIN_TOKEN_LEN {
        set.insert(token.to_ascii_lowercase());
    }
}

fn push_token_bloom(token: &str, bloom: &mut TokenBloom) {
    if token.len() >= MIN_TOKEN_LEN {
        let lower = token.to_ascii_lowercase();
        insert_token(bloom, &lower);
    }
}

fn probe_index(token: &str, probe: u8) -> usize {
    let mut hash = fnv1a(token.as_bytes());
    hash ^= (probe as u64).wrapping_mul(0x9e3779b97f4a7c15);
    (hash % TOKEN_BLOOM_BITS as u64) as usize
}

fn set_bit(bloom: &mut TokenBloom, bit_idx: usize) {
    let word = bit_idx / 64;
    let shift = bit_idx % 64;
    bloom[word] |= 1u64 << shift;
}

fn test_bit(bloom: &TokenBloom, bit_idx: usize) -> bool {
    let word = bit_idx / 64;
    let shift = bit_idx % 64;
    (bloom[word] >> shift) & 1 == 1
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bloom_detects_inserted_body_token() {
        let bloom = build_token_bloom("helper", None, None, Some("let port = ntohs(raw);"));
        assert!(keyword_in_bloom("ntohs", &bloom));
    }

    #[test]
    fn keyword_and_requires_each_term() {
        let bloom = build_token_bloom(
            "processInvoice",
            Some("InvoiceService.processInvoice"),
            None,
            Some("posting to GL account"),
        );
        assert!(satisfies_keyword_and(
            &["invoice".into(), "posting".into()],
            &bloom
        ));
        assert!(!satisfies_keyword_and(
            &["invoice".into(), "payment".into()],
            &bloom
        ));
    }
}
