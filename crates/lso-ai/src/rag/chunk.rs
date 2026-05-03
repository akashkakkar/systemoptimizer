//! Paragraph-aware chunker.
//!
//! Targets ~512 tokens per chunk with ~50 tokens of overlap, matching
//! F17's spec. Token counts are approximated from character length so
//! we don't pin the system to a particular tokenizer.

const APPROX_CHARS_PER_TOKEN: usize = 4;
pub const CHUNK_TOKEN_SIZE: usize = 512;
pub const CHUNK_OVERLAP_TOKENS: usize = 50;

/// Split `text` into chunks. Paragraphs (separated by blank lines) are
/// kept together when possible. Paragraphs longer than the target size
/// are hard-wrapped at character boundaries. Each chunk after the
/// first is prefixed with the tail of the previous chunk to provide
/// overlap.
pub fn chunk_paragraphs(text: &str) -> Vec<String> {
    let max_chars = CHUNK_TOKEN_SIZE * APPROX_CHARS_PER_TOKEN;
    let overlap_chars = CHUNK_OVERLAP_TOKENS * APPROX_CHARS_PER_TOKEN;

    let paragraphs: Vec<String> = text
        .split("\n\n")
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();

    let mut chunks: Vec<String> = Vec::new();
    let mut buf = String::new();

    for para in paragraphs {
        if para.len() > max_chars {
            if !buf.is_empty() {
                chunks.push(std::mem::take(&mut buf));
            }
            hard_wrap_into(&para, max_chars, overlap_chars, &mut chunks);
            continue;
        }

        let separator = if buf.is_empty() { 0 } else { 2 };
        if buf.len() + separator + para.len() <= max_chars {
            if !buf.is_empty() {
                buf.push_str("\n\n");
            }
            buf.push_str(&para);
        } else {
            chunks.push(std::mem::take(&mut buf));
            buf.push_str(&para);
        }
    }
    if !buf.is_empty() {
        chunks.push(buf);
    }

    if chunks.len() <= 1 || overlap_chars == 0 {
        return chunks;
    }

    let mut out = Vec::with_capacity(chunks.len());
    out.push(chunks[0].clone());
    for i in 1..chunks.len() {
        let prev = &chunks[i - 1];
        let mut start = prev.len().saturating_sub(overlap_chars);
        while start < prev.len() && !prev.is_char_boundary(start) {
            start += 1;
        }
        let tail = prev[start..].trim();
        if tail.is_empty() {
            out.push(chunks[i].clone());
        } else {
            out.push(format!("{}\n\n{}", tail, chunks[i]));
        }
    }
    out
}

fn hard_wrap_into(s: &str, max: usize, overlap: usize, out: &mut Vec<String>) {
    let bytes = s.as_bytes();
    let mut start = 0;
    while start < bytes.len() {
        let mut end = (start + max).min(bytes.len());
        while end > start && !s.is_char_boundary(end) {
            end -= 1;
        }
        out.push(s[start..end].to_string());
        if end == bytes.len() {
            break;
        }
        let mut next = end.saturating_sub(overlap);
        while next < bytes.len() && !s.is_char_boundary(next) {
            next += 1;
        }
        start = next.max(start + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_no_chunks() {
        assert!(chunk_paragraphs("").is_empty());
        assert!(chunk_paragraphs("\n\n\n").is_empty());
    }

    #[test]
    fn small_text_is_single_chunk() {
        let chunks = chunk_paragraphs("hello world");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "hello world");
    }

    #[test]
    fn paragraphs_pack_under_limit() {
        let p1 = "para one";
        let p2 = "para two";
        let p3 = "para three";
        let text = format!("{}\n\n{}\n\n{}", p1, p2, p3);
        let chunks = chunk_paragraphs(&text);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].contains(p1) && chunks[0].contains(p2) && chunks[0].contains(p3));
    }

    #[test]
    fn long_text_splits_with_overlap() {
        // Build a doc clearly larger than one chunk
        let para = "x".repeat(CHUNK_TOKEN_SIZE * APPROX_CHARS_PER_TOKEN - 10);
        let text = (0..5)
            .map(|i| format!("paragraph_{} {}", i, para))
            .collect::<Vec<_>>()
            .join("\n\n");
        let chunks = chunk_paragraphs(&text);
        assert!(chunks.len() > 1, "expected multiple chunks, got {}", chunks.len());
        // No chunk drastically over the limit (overlap added on top
        // expands the chunk a bit, so allow some slack).
        let max_with_overlap = (CHUNK_TOKEN_SIZE + CHUNK_OVERLAP_TOKENS + 10) * APPROX_CHARS_PER_TOKEN;
        for c in &chunks {
            assert!(c.len() <= max_with_overlap, "chunk len {}", c.len());
        }
    }

    #[test]
    fn hard_wrap_handles_oversized_paragraph() {
        let big = "a".repeat(CHUNK_TOKEN_SIZE * APPROX_CHARS_PER_TOKEN * 3);
        let chunks = chunk_paragraphs(&big);
        assert!(chunks.len() >= 3);
    }

    #[test]
    fn unicode_safe() {
        let p = "héllo wörld ".repeat(500);
        let chunks = chunk_paragraphs(&p);
        // Just check we didn't panic on char boundaries
        assert!(!chunks.is_empty());
        for c in chunks {
            // Round-trip: must be valid UTF-8 (it is, by construction)
            assert!(c.is_char_boundary(0));
        }
    }
}
