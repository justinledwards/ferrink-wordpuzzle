use std::cell::RefCell;
use std::collections::HashMap;

use super::traits::WordRepository;

struct Inner {
    cache: HashMap<usize, Vec<String>>,
}

pub struct WordRepositoryImpl {
    inner: RefCell<Inner>,
}

impl WordRepositoryImpl {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(Inner {
                cache: HashMap::new(),
            }),
        }
    }
}

impl Default for WordRepositoryImpl {
    fn default() -> Self {
        Self::new()
    }
}

fn embedded_words(length: usize) -> Option<&'static [u8]> {
    match length {
        5 => Some(crate::word_data::WORDS_5),
        _ => None,
    }
}

fn decompress(data: &[u8]) -> Vec<String> {
    let decompressed = match zstd::decode_all(std::io::Cursor::new(data)) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("WordRepository: decompress error: {e}");
            return Vec::new();
        }
    };
    let text = String::from_utf8_lossy(&decompressed);
    text.lines()
        .filter_map(|s| {
            let trimmed = s.trim().to_lowercase();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect()
}

impl WordRepository for WordRepositoryImpl {
    fn get_words(&self, length: usize) -> Vec<String> {
        let mut inner = self.inner.borrow_mut();
        if let Some(words) = inner.cache.get(&length) {
            return words.clone();
        }

        let Some(data) = embedded_words(length) else {
            return Vec::new();
        };

        let words = decompress(data);
        inner.cache.insert(length, words.clone());
        words
    }

    fn unload_words(&self, length: usize) {
        let mut inner = self.inner.borrow_mut();
        inner.cache.remove(&length);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_words_6() {
        let repo = WordRepositoryImpl::new();
        let words = repo.get_words(5);
        assert!(!words.is_empty(), "Should load 5-letter words");
        assert!(words.iter().all(|w| w.len() == 5), "All words should be 5 letters");
    }

    #[test]
    fn test_missing_length() {
        let repo = WordRepositoryImpl::new();
        let words = repo.get_words(99);
        assert!(words.is_empty(), "Missing length should return empty");
    }
}
