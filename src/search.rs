use nucleo::{Matcher, Utf32Str};

use crate::backends::{SearchBackend, SearchResult};

fn fuzzy_match_safe(matcher: &mut Matcher, haystack: Utf32Str, needle: Utf32Str) -> Option<u16> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        matcher.fuzzy_match(haystack, needle)
    }));
    result.ok().flatten()
}

pub struct SearchEngine {
    backends: Vec<Box<dyn SearchBackend>>,
    matcher: Matcher,
}

impl SearchEngine {
    pub fn new(backends: Vec<Box<dyn SearchBackend>>) -> Self {
        Self {
            backends,
            matcher: Matcher::default(),
        }
    }

    pub fn search(&mut self, query: &str, max_results: usize) -> Vec<GroupedResults> {
        if query.is_empty() {
            return vec![];
        }

        let mut needle_buf = Vec::new();
        let needle = Utf32Str::new(query, &mut needle_buf);
        let mut hay_buf = Vec::new();
        let mut grouped: Vec<GroupedResults> = Vec::new();

        for backend in &self.backends {
            let mut results = backend.search(query, max_results * 2);
            if results.is_empty() {
                continue;
            }

            for r in &mut results {
                hay_buf.clear();
                let haystack = Utf32Str::new(&r.title, &mut hay_buf);
                r.score = match fuzzy_match_safe(&mut self.matcher, haystack, needle) {
                    Some(s) => s as i64,
                    None => {
                        if let Some(sub) = &r.subtitle {
                            hay_buf.clear();
                            let haystack = Utf32Str::new(sub, &mut hay_buf);
                            fuzzy_match_safe(&mut self.matcher, haystack, needle)
                                .map(|s| s as i64)
                                .unwrap_or(0)
                        } else {
                            0
                        }
                    }
                };
            }

            results.sort_by(|a, b| b.score.cmp(&a.score));
            results.retain(|r| r.score > 0);
            results.truncate(max_results);

            if !results.is_empty() {
                grouped.push(GroupedResults {
                    name: backend.name().to_string(),
                    icon: backend.icon().to_string(),
                    results,
                });
            }
        }

        grouped
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GroupedResults {
    pub name: String,
    pub icon: String,
    pub results: Vec<SearchResult>,
}
