/// Find case-insensitive matches of `query` in `text`, returning byte-offset pairs
/// into `text` that are guaranteed valid for slicing.
/// Uses character-level case folding to avoid to_lowercase() byte-length mismatch.
pub fn find_matches_case_insensitive<'a>(text: &'a str, query: &str) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }

    let text_chars: Vec<(usize, char)> = text.char_indices().collect();
    let query_lower: Vec<char> = query.chars().flat_map(|c| c.to_lowercase()).collect();
    let text_lower: Vec<char> = text
        .chars()
        .map(|c| c.to_lowercase().next().unwrap_or(c))
        .collect();

    let text_len = text_chars.len();
    let q_len = query_lower.len();
    let mut matches = Vec::new();
    let mut i = 0;
    while i + q_len <= text_len {
        if text_lower[i..i + q_len] == query_lower[..] {
            let byte_start = text_chars[i].0;
            let byte_end = if i + q_len < text_len {
                text_chars[i + q_len].0
            } else {
                text.len()
            };
            matches.push((byte_start, byte_end));
            i += q_len;
        } else {
            i += 1;
        }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::find_matches_case_insensitive;

    #[test]
    fn test_find_matches_ascii() {
        let m = find_matches_case_insensitive("deploy backend", "deploy");
        assert_eq!(m, vec![(0, 6)]);
    }

    #[test]
    fn test_find_matches_case_insensitive_ascii() {
        let m = find_matches_case_insensitive("Deploy Backend", "deploy");
        assert_eq!(m, vec![(0, 6)]);
    }

    #[test]
    fn test_find_matches_no_match() {
        let m = find_matches_case_insensitive("hello world", "xyz");
        assert!(m.is_empty());
    }

    #[test]
    fn test_find_matches_empty_query() {
        let m = find_matches_case_insensitive("hello", "");
        assert!(m.is_empty());
    }

    #[test]
    fn test_find_matches_multiple() {
        let m = find_matches_case_insensitive("test test test", "test");
        assert_eq!(m.len(), 3);
        assert_eq!(m[0], (0, 4));
        assert_eq!(m[1], (5, 9));
        assert_eq!(m[2], (10, 14));
    }

    #[test]
    fn test_find_matches_partial_word() {
        // "deployment" — "ploy" starts at char index 2 (byte 2)
        let m = find_matches_case_insensitive("deployment", "ploy");
        assert_eq!(m, vec![(2, 6)]);
    }

    #[test]
    fn test_find_matches_unicode_safe() {
        // Use characters whose case-folding does NOT change byte length
        let m = find_matches_case_insensitive("Café", "café");
        assert_eq!(m, vec![(0, 5)]);
    }

    #[test]
    fn test_find_matches_eszett_roundtrip() {
        // ẞ (U+1E9E, capital sharp S, 3 bytes in UTF-8) → ß (U+00DF, 2 bytes)
        // The match byte positions come from char_indices of the original text
        let text = "STRAẞE";
        // Search for "E" at the end — should only match the last character
        let m = find_matches_case_insensitive(text, "e");
        assert_eq!(m.len(), 1);
        // The match should be the last character "E" (byte 6..7)
        assert_eq!(&text[m[0].0..m[0].1], "E");
    }

    #[test]
    fn test_find_matches_eszett_inside() {
        // ẞ to ß changes byte length: 3 bytes → 2 bytes
        // This test verifies we don't panic on such strings
        let text = "STRAẞE";
        let m = find_matches_case_insensitive(text, "e");
        assert!(!m.is_empty());
        for (start, end) in &m {
            // Every slice should be valid UTF-8 (will not panic)
            let _ = &text[*start..*end];
        }
    }
}
