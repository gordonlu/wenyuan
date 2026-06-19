use async_trait::async_trait;
use scraper::{Html, Selector};
use std::time::Duration;
use wenyuan_core::{SearchBackend, SearchError, SearchResult};

pub struct BingBackend {
    client: reqwest::Client,
    delay: Duration,
}

impl BingBackend {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            delay: Duration::from_millis(1500),
        }
    }
}

#[async_trait]
impl SearchBackend for BingBackend {
    fn name(&self) -> &'static str {
        "bing"
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let url = format!("https://cn.bing.com/search?q={}", urlencoding(query));
        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
            )
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.5")
            .send()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;

        let results = tokio::task::spawn_blocking(move || parse_bing(&body, limit))
            .await
            .map_err(|e| SearchError::Request(e.to_string()))??;

        tokio::time::sleep(self.delay).await;
        Ok(results)
    }
}

fn parse_bing(body: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
    let doc = Html::parse_document(body);
    let li_sel = Selector::parse("li.b_algo").map_err(|e| SearchError::Request(e.to_string()))?;
    let a_sel = Selector::parse("h2 a").map_err(|e| SearchError::Request(e.to_string()))?;
    let p_sel =
        Selector::parse(".b_caption p").map_err(|e| SearchError::Request(e.to_string()))?;

    let mut results = Vec::new();
    for li in doc.select(&li_sel).take(limit) {
        let title = li
            .select(&a_sel)
            .next()
            .map(|a| a.text().collect::<String>())
            .unwrap_or_default();
        let url = li
            .select(&a_sel)
            .next()
            .and_then(|a| a.value().attr("href"))
            .unwrap_or("")
            .to_string();
        let snippet = li
            .select(&p_sel)
            .next()
            .map(|p| p.text().collect::<String>())
            .unwrap_or_default();
        if !title.is_empty() {
            results.push(SearchResult {
                title,
                snippet,
                url,
                source: "bing".into(),
            });
        }
    }
    Ok(results)
}

impl Default for BingBackend {
    fn default() -> Self {
        Self::new()
    }
}

pub struct WikipediaBackend {
    client: reqwest::Client,
    api_url: String,
}

impl WikipediaBackend {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            api_url: "https://zh.wikipedia.org/w/api.php".into(),
        }
    }

    pub fn with_api_url(mut self, url: String) -> Self {
        self.api_url = url;
        self
    }
}

#[async_trait]
impl SearchBackend for WikipediaBackend {
    fn name(&self) -> &'static str {
        "wikipedia"
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let resp = self
            .client
            .get(&self.api_url)
            .query(&[
                ("action", "query"),
                ("list", "search"),
                ("format", "json"),
                ("srsearch", query),
                ("srlimit", &limit.to_string()),
                ("utf8", "1"),
            ])
            .send()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;

        let results = data["query"]["search"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|item| {
                        let title = item["title"].as_str().unwrap_or("").to_string();
                        let snippet = item["snippet"]
                            .as_str()
                            .unwrap_or("")
                            .replace("<span class=\"searchmatch\">", "")
                            .replace("</span>", "");
                        let page_id = item["pageid"].as_i64().unwrap_or(0);
                        SearchResult {
                            title,
                            snippet,
                            url: format!("https://zh.wikipedia.org/wiki?curid={page_id}"),
                            source: "wikipedia".into(),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }
}

impl Default for WikipediaBackend {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DuckDuckGoBackend {
    client: reqwest::Client,
    delay: Duration,
}

impl DuckDuckGoBackend {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            delay: Duration::from_millis(1000),
        }
    }
}

#[async_trait]
impl SearchBackend for DuckDuckGoBackend {
    fn name(&self) -> &'static str {
        "duckduckgo"
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let url = format!("https://lite.duckduckgo.com/lite/?q={}", urlencoding(query));
        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;

        let results = tokio::task::spawn_blocking(move || parse_ddg(&body, limit))
            .await
            .map_err(|e| SearchError::Request(e.to_string()))??;

        tokio::time::sleep(self.delay).await;
        Ok(results)
    }
}

fn parse_ddg(body: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
    let doc = Html::parse_document(body);
    let result_sel =
        Selector::parse(".result-link").map_err(|e| SearchError::Request(e.to_string()))?;
    let snippet_sel =
        Selector::parse(".result-snippet").map_err(|e| SearchError::Request(e.to_string()))?;

    let mut results = Vec::new();
    let links: Vec<_> = doc.select(&result_sel).collect();
    let snippets: Vec<_> = doc.select(&snippet_sel).collect();

    for (link, snippet) in links.iter().zip(snippets.iter()).take(limit) {
        let title = link.text().collect::<String>();
        let url = link
            .value()
            .attr("href")
            .unwrap_or("")
            .to_string();
        let snippet_text = snippet.text().collect::<String>();
        if !title.is_empty() {
            results.push(SearchResult {
                title,
                snippet: snippet_text,
                url,
                source: "duckduckgo".into(),
            });
        }
    }
    Ok(results)
}

impl Default for DuckDuckGoBackend {
    fn default() -> Self {
        Self::new()
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
        let req = self.client.get(&self.url).query(&[
            ("q", query),
            ("limit", &limit.to_string()),
        ]);
        let req = if let Some(key) = &self.api_key {
            req.header("Authorization", format!("Bearer {key}"))
        } else {
            req
        };
        let resp = req
            .send()
            .await
            .map_err(|e| SearchError::Request(e.to_string()))?;
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
}

#[async_trait]
impl SearchBackend for SearchPool {
    fn name(&self) -> &'static str {
        "pool"
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let mut all = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();
        for backend in &self.backends {
            if let Ok(results) = backend.search(query, limit).await {
                for r in results {
                    if seen_urls.insert(r.url.clone()) {
                        all.push(r);
                    }
                }
            }
        }
        all.truncate(limit);
        Ok(all)
    }
}

fn urlencoding(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            other => format!("%{:02X}", other as u8),
        })
        .collect()
}
