// crates/fluent-core/src/utils.rs
pub fn some_utility_function() {
    // Utility function implementation
}


pub mod chunking {
    use unicode_segmentation::UnicodeSegmentation;

    pub const CHUNK_SIZE: usize = 1000; // Adjust this value as needed
    pub const CHUNK_OVERLAP: usize = 200; // Adjust this value as needed

    pub fn chunk_document(content: &str) -> Vec<String> {
        let words: Vec<&str> = content.unicode_words().collect();
        let mut chunks = Vec::new();
        let mut start = 0;

        while start < words.len() {
            let end = (start + CHUNK_SIZE).min(words.len());
            let chunk = words[start..end].join(" ");
            chunks.push(chunk);

            if end == words.len() {
                break;
            }

            start = if end > CHUNK_OVERLAP { end - CHUNK_OVERLAP } else { 0 };
        }

        chunks
    }
}