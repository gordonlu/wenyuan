use async_trait::async_trait;
use std::time::Duration;
use wenyuan_core::{SearchBackend, SearchError, SearchResult};

const MAX_QUERY_CHARS: usize = 2000;

fn truncate_query(query: &str) -> &str {
    if query.chars().count() > MAX_QUERY_CHARS {
        let end = query
            .char_indices()
            .nth(MAX_QUERY_CHARS)
            .map(|(i, _)| i)
            .unwrap_or(query.len());
        &query[..end]
    } else {
        query
    }
}

pub struct CustomSearchBackend {
    client: reqwest::Client,
    url: String,
    api_key: Option<String>,
}

impl CustomSearchBackend {
    pub fn new(url: String, api_key: Option<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            url,
            api_key,
        }
    }
}

#[async_trait]
impl SearchBackend for CustomSearchBackend {
    fn name(&self) -> &'static str {
        "custom"
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let req = self
            .client
            .get(&self.url)
            .query(&[("q", query), ("limit", &limit.to_string())]);
        let req = if let Some(key) = &self.api_key {
            req.header("Authorization", format!("Bearer {key}"))
        } else {
            req
        };
        let resp = req
            .send()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;
        ensure_success(self.name(), resp.status())?;
        let results: Vec<SearchResult> = resp
            .json()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;
        Ok(results)
    }
}

/// 豆包网页搜索 API (Volc / feedcoop)
/// API: POST https://open.feedcoopapi.com/search_api/web_search
/// Env: WENYUAN_SEARCH_DOUBAO_KEY
pub struct DoubaoBackend {
    client: reqwest::Client,
    api_key: String,
}

impl DoubaoBackend {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            api_key,
        }
    }
}

#[async_trait]
impl SearchBackend for DoubaoBackend {
    fn name(&self) -> &'static str {
        "doubao"
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let resp = self
            .client
            .post("https://open.feedcoopapi.com/search_api/web_search")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "Query": query,
                "SearchType": "web",
                "Count": limit.min(50),
                "Filter": {
                    "NeedContent": false,
                    "NeedUrl": true
                },
                "NeedSummary": true,
            }))
            .send()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;
        ensure_success(self.name(), resp.status())?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;

        let results = data["Result"]["WebResults"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|item| SearchResult {
                        title: item["Title"].as_str().unwrap_or("").to_string(),
                        snippet: item["Summary"]
                            .as_str()
                            .or_else(|| item["Snippet"].as_str())
                            .unwrap_or("")
                            .to_string(),
                        url: item["Url"].as_str().unwrap_or("").to_string(),
                        source: "doubao".into(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }
}

/// Tavily AI Search API
/// Docs: https://docs.tavily.com/
/// Env: WENYUAN_SEARCH_TAVILY_KEY
pub struct TavilyBackend {
    client: reqwest::Client,
    api_key: String,
}

impl TavilyBackend {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            api_key,
        }
    }
}

#[async_trait]
impl SearchBackend for TavilyBackend {
    fn name(&self) -> &'static str {
        "tavily"
    }

    async fn search(&self, query: &str, _limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let resp = self
            .client
            .post("https://api.tavily.com/search")
            .json(&serde_json::json!({
                "api_key": self.api_key,
                "query": query,
                "max_results": _limit,
            }))
            .send()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;
        ensure_success(self.name(), resp.status())?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;

        let results = data["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|item| SearchResult {
                        title: item["title"].as_str().unwrap_or("").to_string(),
                        snippet: item["content"].as_str().unwrap_or("").to_string(),
                        url: item["url"].as_str().unwrap_or("").to_string(),
                        source: "tavily".into(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }
}

/// Google Custom Search API
/// Docs: https://developers.google.com/custom-search/v1/overview
/// Env: WENYUAN_SEARCH_GOOGLE_KEY, WENYUAN_SEARCH_GOOGLE_CX
pub struct GoogleCustomSearchBackend {
    client: reqwest::Client,
    api_key: String,
    cx: String,
}

impl GoogleCustomSearchBackend {
    pub fn new(api_key: String, cx: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            api_key,
            cx,
        }
    }
}

#[async_trait]
impl SearchBackend for GoogleCustomSearchBackend {
    fn name(&self) -> &'static str {
        "google"
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let resp = self
            .client
            .get("https://www.googleapis.com/customsearch/v1")
            .query(&[
                ("key", &self.api_key),
                ("cx", &self.cx),
                ("q", &query.to_string()),
                ("num", &limit.to_string()),
            ])
            .send()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;
        ensure_success(self.name(), resp.status())?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;

        let results = data["items"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|item| SearchResult {
                        title: item["title"].as_str().unwrap_or("").to_string(),
                        snippet: item["snippet"].as_str().unwrap_or("").to_string(),
                        url: item["link"].as_str().unwrap_or("").to_string(),
                        source: "google".into(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }
}

/// SearXNG self-hosted search aggregator
/// Docs: https://docs.searxng.org/
/// Env: WENYUAN_SEARCH_SEARXNG_URL
pub struct SearXNGSearchBackend {
    client: reqwest::Client,
    url: String,
}

impl SearXNGSearchBackend {
    pub fn new(url: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            url,
        }
    }
}

#[async_trait]
impl SearchBackend for SearXNGSearchBackend {
    fn name(&self) -> &'static str {
        "searxng"
    }

    async fn search(&self, query: &str, _limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let resp = self
            .client
            .get(&self.url)
            .query(&[
                ("q", query),
                ("format", "json"),
                ("language", "zh-CN"),
                ("categories", "general"),
            ])
            .send()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;
        ensure_success(self.name(), resp.status())?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;

        let results = data["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|item| SearchResult {
                        title: item["title"].as_str().unwrap_or("").to_string(),
                        snippet: item["content"]
                            .as_str()
                            .or_else(|| item["snippet"].as_str())
                            .unwrap_or("")
                            .to_string(),
                        url: item["url"].as_str().unwrap_or("").to_string(),
                        source: "searxng".into(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }
}

pub struct SearchPool {
    backends: Vec<Box<dyn SearchBackend>>,
}

impl SearchPool {
    pub fn new(backends: Vec<Box<dyn SearchBackend>>) -> Self {
        Self { backends }
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.backends.iter().map(|b| b.name()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.backends.is_empty()
    }

    pub fn backends(&self) -> &[Box<dyn SearchBackend>] {
        &self.backends
    }

    /// Search each backend individually and return results grouped by backend name.
    pub async fn search_grouped(
        &self,
        query: &str,
        per_backend: usize,
    ) -> std::collections::HashMap<String, (Vec<SearchResult>, Option<String>)> {
        let query = truncate_query(query);
        let mut map = std::collections::HashMap::new();
        for backend in &self.backends {
            match backend.search(query, per_backend).await {
                Ok(results) => {
                    map.insert(backend.name().to_string(), (results, None));
                }
                Err(err) => {
                    map.insert(backend.name().to_string(), (Vec::new(), Some(err.to_string())));
                }
            }
        }
        map
    }
}

#[async_trait]
impl SearchBackend for SearchPool {
    fn name(&self) -> &'static str {
        "pool"
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let query = truncate_query(query);
        let mut all = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();
        let mut failures = Vec::new();
        for backend in &self.backends {
            match backend.search(query, limit).await {
                Ok(results) if results.is_empty() => {
                    failures.push(format!("{}: no results", backend.name()));
                }
                Ok(results) => {
                    for r in results {
                        if !r.title.trim().is_empty()
                            && !r.url.trim().is_empty()
                            && seen_urls.insert(r.url.clone())
                        {
                            all.push(r);
                        }
                    }
                }
                Err(err) => failures.push(format!("{}: {err}", backend.name())),
            }
        }
        all.truncate(limit);
        if all.is_empty() {
            Err(SearchError::Backend("pool", failures.join("; ")))
        } else {
            Ok(all)
        }
    }
}

fn ensure_success(backend: &'static str, status: reqwest::StatusCode) -> Result<(), SearchError> {
    if status.is_success() {
        Ok(())
    } else {
        Err(SearchError::Backend(backend, format!("http {status}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    enum StaticOutcome {
        Ok(Vec<SearchResult>),
        Err(&'static str),
    }

    struct StaticSearchBackend {
        name: &'static str,
        result: StaticOutcome,
    }

    #[async_trait]
    impl SearchBackend for StaticSearchBackend {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn search(
            &self,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<SearchResult>, SearchError> {
            match &self.result {
                StaticOutcome::Ok(results) => Ok(results.clone()),
                StaticOutcome::Err(message) => Err(SearchError::Request((*message).into())),
            }
        }
    }

    fn result(url: &str) -> SearchResult {
        SearchResult {
            title: "title".into(),
            snippet: "snippet".into(),
            url: url.into(),
            source: "test".into(),
        }
    }

    #[tokio::test]
    async fn search_pool_returns_first_available_backend() {
        let pool = SearchPool::new(vec![
            Box::new(StaticSearchBackend {
                name: "broken",
                result: StaticOutcome::Err("offline"),
            }),
            Box::new(StaticSearchBackend {
                name: "empty",
                result: StaticOutcome::Ok(vec![]),
            }),
            Box::new(StaticSearchBackend {
                name: "ok",
                result: StaticOutcome::Ok(vec![result("https://example.com")]),
            }),
        ]);

        let results = pool.search("query", 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://example.com");
    }

    #[tokio::test]
    async fn search_pool_errors_when_every_backend_fails_or_is_empty() {
        let pool = SearchPool::new(vec![
            Box::new(StaticSearchBackend {
                name: "broken",
                result: StaticOutcome::Err("offline"),
            }),
            Box::new(StaticSearchBackend {
                name: "empty",
                result: StaticOutcome::Ok(vec![]),
            }),
        ]);

        let err = pool.search("query", 5).await.unwrap_err();
        let message = err.to_string();
        assert!(message.contains("broken"));
        assert!(message.contains("empty"));
    }
}
