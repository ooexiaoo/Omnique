use crate::backends::{SearchBackend, SearchResult};

pub struct SearchEngine {
    backends: Vec<Box<dyn SearchBackend>>,
}

impl SearchEngine {
    pub fn new(backends: Vec<Box<dyn SearchBackend>>) -> Self {
        Self { backends }
    }

    pub fn search(&self, query: &str, max_results: usize) -> Vec<GroupedResults> {
        if query.is_empty() {
            return vec![];
        }

        let mut grouped: Vec<GroupedResults> = Vec::new();

        for backend in &self.backends {
            let results = backend.search(query, max_results);
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
