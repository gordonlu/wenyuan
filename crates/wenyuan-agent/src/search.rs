use wenyuan_core::SearchResult;

#[cfg(test)]
use wenyuan_core::{SearchBackend, SearchError};

pub(crate) const SEARCH_TOOL_INSTRUCTION: &str = "\n\n你可以搜索互联网来获取信息。如果需要搜索，输出 JSON：{\"tool\":\"search\",\"query\":\"你的搜索词\"}。收到搜索结果后，如果还需要搜索可以继续输出工具调用，否则输出你的阶段输出。";

pub(crate) fn try_extract_search_tool(content: &str) -> Option<String> {
    let val: serde_json::Value = serde_json::from_str(content.trim()).ok()?;
    if val.get("tool")?.as_str()? != "search" {
        return None;
    }
    let query = val.get("query")?.as_str()?;
    let trimmed = query.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(crate) fn format_search_results(results: &[SearchResult], query: &str) -> String {
    if results.is_empty() {
        return format!(
            "搜索 [{}] 无结果。如果需要换词搜索请继续输出工具调用。",
            query
        );
    }
    let lines: Vec<String> = results
        .iter()
        .map(|r| format!("- {}\n  {}\n  {} ({})", r.title, r.snippet, r.url, r.source))
        .collect();
    format!(
        "搜索 [{}] 的结果：\n{}\n\n如果还需要搜索请输出工具调用 JSON，否则输出你的阶段输出。",
        query,
        lines.join("\n")
    )
}

#[cfg(test)]
pub struct MockSearchBackend {
    pub results: Vec<SearchResult>,
}

#[cfg(test)]
impl MockSearchBackend {
    pub fn new() -> Self {
        Self {
            results: vec![
                SearchResult {
                    title: "文渊阁项目介绍".into(),
                    snippet: "文渊阁是一个本地运行的 AI 合议工作台，把同一个问题交给三个不同立场的席位分别思考、互相批议、修订方案，并通过投票形成最终结论。".into(),
                    url: "https://github.com/gordonlu/wenyuan".into(),
                    source: "mock".into(),
                },
                SearchResult {
                    title: "AI 合议与多数决机制".into(),
                    snippet: "三席合议机制包括独议、批议、复议、阁议四个阶段，支持多数决和少数留议。".into(),
                    url: "https://example.com/deliberation".into(),
                    source: "mock".into(),
                },
                SearchResult {
                    title: "三席角色设计：谋远、经世、持正".into(),
                    snippet: "谋远席负责长期战略和系统性思考，经世席关注落地路径和资源约束，持正席审查风险、伦理和边界条件。".into(),
                    url: "https://example.com/three-seats".into(),
                    source: "mock".into(),
                },
            ],
        }
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl SearchBackend for MockSearchBackend {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn search(&self, _query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        Ok(self.results.iter().take(limit).cloned().collect())
    }
}
