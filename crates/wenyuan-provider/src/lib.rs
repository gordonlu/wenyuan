use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;
use wenyuan_core::{ChatMessage, SeatKind, SessionPhase};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub session_id: Uuid,
    pub seat: SeatKind,
    pub phase: SessionPhase,
    pub messages: Vec<ChatMessage>,
    pub repair_json: bool,
    pub max_tokens: u32,
    pub prompt_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub usage: Option<TokenUsage>,
    pub upstream_status: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider timeout")]
    Timeout,
    #[error("provider cancelled")]
    Cancelled,
    #[error("provider request failed: {0}")]
    Request(String),
    #[error("provider request failed with upstream status {status}: {message}")]
    HttpStatus { status: u16, message: String },
    #[error("provider returned invalid response: {0}")]
    InvalidResponse(String),
}

impl ProviderError {
    pub fn upstream_status(&self) -> Option<u16> {
        match self {
            ProviderError::HttpStatus { status, .. } => Some(*status),
            _ => None,
        }
    }
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, ProviderError>;
}

#[derive(Clone)]
pub struct SeatRoutedProvider {
    default: Arc<dyn LlmProvider>,
    seat_providers: HashMap<SeatKind, Arc<dyn LlmProvider>>,
}

impl SeatRoutedProvider {
    pub fn new(default: Arc<dyn LlmProvider>) -> Self {
        Self {
            default,
            seat_providers: HashMap::new(),
        }
    }

    pub fn with_seat_provider(mut self, seat: SeatKind, provider: Arc<dyn LlmProvider>) -> Self {
        self.seat_providers.insert(seat, provider);
        self
    }
}

#[async_trait]
impl LlmProvider for SeatRoutedProvider {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, ProviderError> {
        let provider = self
            .seat_providers
            .get(&request.seat)
            .unwrap_or(&self.default)
            .clone();
        provider.complete(request).await
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub reasoning_effort: Option<String>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProvider {
    config: OpenAiCompatibleConfig,
    client: reqwest::Client,
}

impl OpenAiCompatibleProvider {
    pub fn new(config: OpenAiCompatibleConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct OpenAiRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    usage: Option<TokenUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: ChatMessage,
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, ProviderError> {
        let model = request
            .override_model
            .as_deref()
            .unwrap_or(&self.config.model);
        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );
        let body = OpenAiRequest {
            model,
            messages: &request.messages,
            max_tokens: self
                .config
                .max_tokens
                .map(|configured| configured.min(request.max_tokens))
                .unwrap_or(request.max_tokens),
            reasoning_effort: normalized_reasoning_effort(
                model,
                request
                    .reasoning_effort
                    .as_deref()
                    .or(self.config.reasoning_effort.as_deref()),
            ),
        };
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| ProviderError::Request(err.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            let message = response.text().await.unwrap_or_default();
            return Err(ProviderError::HttpStatus {
                status: status.as_u16(),
                message,
            });
        }
        let body_bytes = response
            .bytes()
            .await
            .map_err(|err| ProviderError::InvalidResponse(format!("read body failed: {err}")))?;
        if body_bytes.is_empty() {
            return Err(ProviderError::InvalidResponse("upstream returned empty body".into()));
        }
        let payload: OpenAiResponse = serde_json::from_slice(&body_bytes)
            .map_err(|err| ProviderError::InvalidResponse(format!("parse failed: {err}")))?;
        let content = payload
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or_else(|| ProviderError::InvalidResponse("missing choice".into()))?;
        Ok(LlmResponse {
            content,
            usage: payload.usage,
            upstream_status: Some(status.as_u16()),
        })
    }
}

fn normalized_reasoning_effort(model: &str, effort: Option<&str>) -> Option<&'static str> {
    let model = model.to_ascii_lowercase();
    let raw = effort
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("none"));

    if model.contains("deepseek") {
        return match raw.map(|value| value.to_ascii_lowercase()).as_deref() {
            Some("max") | Some("xhigh") | Some("extra_high") => Some("max"),
            Some("low") | Some("medium") | Some("high") => Some("high"),
            Some("auto") | None => None,
            Some(_) => Some("high"),
        };
    }

    match raw.map(|value| value.to_ascii_lowercase()).as_deref() {
        Some("low") => Some("low"),
        Some("medium") => Some("medium"),
        Some("high") => Some("high"),
        Some("xhigh") | Some("max") => Some("high"),
        Some("auto") | None => None,
        Some(_) => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockScenario {
    SuccessMajority,
    Timeout,
    SingleSeatTimeout,
    MalformedThenRepair,
    AlwaysMalformed,
    SingleSeatFailure,
    SplitThenConvergence,
}

#[derive(Debug, Clone)]
pub struct MockProvider {
    scenario: MockScenario,
    malformed_seen: Arc<AtomicUsize>,
}

impl MockProvider {
    pub fn new(scenario: MockScenario) -> Self {
        Self {
            scenario,
            malformed_seen: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn content_for(&self, request: &LlmRequest) -> String {
        if request.prompt_version == "scribe-v1" {
            return scribe_json();
        }
        if request.prompt_version == "search-keywords-v1" && request.phase == SessionPhase::Draft {
            return r#"{"query":"鱼缸 水质 褐藻"}"#.to_string();
        }
        match request.phase {
            SessionPhase::IndependentDeliberation => independent_json(request.seat),
            SessionPhase::CrossCritique => critique_json(request.seat),
            SessionPhase::Revision | SessionPhase::Convergence => proposal_json(request.seat),
            SessionPhase::Voting => vote_json(request.seat, self.scenario),
            _ => "{}".to_string(),
        }
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, ProviderError> {
        match self.scenario {
            MockScenario::Timeout => {
                tokio::time::sleep(Duration::from_millis(500)).await;
                Err(ProviderError::Timeout)
            }
            MockScenario::SingleSeatTimeout if request.seat == SeatKind::Jingshi => {
                tokio::time::sleep(Duration::from_millis(500)).await;
                Err(ProviderError::Timeout)
            }
            MockScenario::SingleSeatFailure if request.seat == SeatKind::Jingshi => {
                Err(ProviderError::Request("mock single seat failure".into()))
            }
            MockScenario::AlwaysMalformed => Ok(LlmResponse {
                content: "{ broken json".into(),
                usage: Some(mock_usage()),
                upstream_status: Some(200),
            }),
            MockScenario::MalformedThenRepair if !request.repair_json => {
                let previous = self.malformed_seen.fetch_add(1, Ordering::SeqCst);
                if previous == 0 {
                    Ok(LlmResponse {
                        content: "{ broken json".into(),
                        usage: Some(mock_usage()),
                        upstream_status: Some(200),
                    })
                } else {
                    Ok(LlmResponse {
                        content: self.content_for(&request),
                        usage: Some(mock_usage()),
                        upstream_status: Some(200),
                    })
                }
            }
            _ => Ok(LlmResponse {
                content: self.content_for(&request),
                usage: Some(mock_usage()),
                upstream_status: Some(200),
            }),
        }
    }
}

fn mock_usage() -> TokenUsage {
    TokenUsage {
        prompt_tokens: 120,
        completion_tokens: 80,
        total_tokens: 200,
    }
}

fn independent_json(seat: SeatKind) -> String {
    serde_json::json!({
        "position": format!("{}主张先形成可验证路径", seat.label()),
        "ideas": [{
            "title": format!("{}初案", seat.label()),
            "summary": "围绕议题给出一个可比较的结构化想法",
            "value": "帮助用户更快看到取舍",
            "mechanism": "独立构思后交叉批议",
            "unconventional": false,
            "assumptions": ["用户愿意提供足够背景"],
            "risks": ["模型输出需要格式校验"]
        }, {
            "title": format!("{}非主流备选", seat.label()),
            "summary": "保留一个与常规路径不同的小规模反向验证方案",
            "value": "避免三席只重复主流答案",
            "mechanism": "用低成本实验验证反直觉假设",
            "unconventional": true,
            "assumptions": ["用户允许探索非默认路径"],
            "risks": ["短期收益可能不明显"]
        }],
        "questions": ["成功标准如何量化"],
        "confidence": 0.75
    })
    .to_string()
}

fn critique_json(seat: SeatKind) -> String {
    let targets: Vec<_> = SeatKind::ALL
        .into_iter()
        .filter(|target| *target != seat)
        .map(|target| {
            serde_json::json!({
                "target_seat": target,
                "strongest_point": "提出了清晰的用户价值",
                "weakest_point": "落地路径还需要收敛",
                "hidden_assumption": "假设数据足够完整",
                "challenge": "请给出最小可行验证",
                "counterexample": "若用户背景不足，该方案可能无法区分优先级",
                "suggested_improvement": "补充验收指标和失败边界",
                "evidence_question": "需要哪些用户反馈或成本数据来验证"
            })
        })
        .collect();
    serde_json::json!({ "reviews": targets }).to_string()
}

fn proposal_json(seat: SeatKind) -> String {
    serde_json::json!({
        "title": format!("{}策案", seat.label()),
        "summary": "以最小闭环先完成一次三席合议",
        "source_idea_ids": [],
        "adopted_points": ["保留可验证路径", "吸收批议中的验收指标"],
        "rejected_points": ["暂不扩大复杂基础设施"],
        "rejection_reasons": ["当前阶段先证明讨论价值"],
        "changes_from_initial": ["从初案补充了失败边界和成功指标"],
        "user_value": "用结构化过程替代松散聊天",
        "implementation_path": "先 Mock 跑通，再接真实 Provider",
        "risks": ["真实模型格式稳定性", "阶段并发控制"],
        "success_metrics": ["完整流程可重复执行", "多数和少数意见可追溯"],
        "confidence": 0.78
    })
    .to_string()
}

fn vote_json(seat: SeatKind, scenario: MockScenario) -> String {
    let choice = match scenario {
        MockScenario::SplitThenConvergence => match seat {
            SeatKind::Mouyuan => "proposal_0",
            SeatKind::Jingshi => "proposal_1",
            SeatKind::Chizheng => "proposal_2",
        },
        _ => "proposal_0",
    };
    let (key_evidence, blocking_issue) = match (scenario, seat) {
        (MockScenario::SplitThenConvergence, SeatKind::Chizheng) => (
            "数据隐私和合规风险需优先评估".to_string(),
            "多数策案未充分处理数据隐私问题".to_string(),
        ),
        _ => (String::new(), String::new()),
    };
    serde_json::json!({
        "votes": [
            {
                "proposal_ref": choice,
                "value_score": 4,
                "novelty_score": 4,
                "feasibility_score": 4,
                "risk_score": 3,
                "roi_score": 4,
                "final_choice": true,
                "reason": format!("{}认为该策案更容易形成闭环", seat.label()),
                "confidence": 0.82,
                "key_evidence": key_evidence,
                "blocking_issue": blocking_issue
            }
        ]
    })
    .to_string()
}

fn scribe_json() -> String {
    serde_json::json!({
      "consensus_summary": "三席均认同需要先完成最小闭环验证",
      "structural_gaps": ["缺少明确的成功指标"],
      "unresolved_conflicts": ["先做企业版还是低代码版本未达成一致"],
      "final_report": "三席合议后，谋远席和经世席支持优先完成内测版，持正席建议补充隐私影响评估。\n\n共识：最小闭环方案应包含明确的验收标准和失败边界。\n\n后续：建议先收集5个内测用户反馈后再决定是否扩展。"
    })
    .to_string()
}

pub mod search;
pub use search::{
    CustomSearchBackend, DoubaoBackend, GoogleCustomSearchBackend,
    SearXNGSearchBackend, SearchPool, TavilyBackend,
};

#[cfg(test)]
mod tests {
    use super::*;

    struct StaticProvider(&'static str);

    #[async_trait]
    impl LlmProvider for StaticProvider {
        async fn complete(&self, _request: LlmRequest) -> Result<LlmResponse, ProviderError> {
            Ok(LlmResponse {
                content: self.0.to_string(),
                usage: None,
                upstream_status: Some(200),
            })
        }
    }

    fn request_for(seat: SeatKind) -> LlmRequest {
        LlmRequest {
            session_id: Uuid::new_v4(),
            seat,
            phase: SessionPhase::IndependentDeliberation,
            messages: vec![],
            repair_json: false,
            max_tokens: 800,
            prompt_version: "test".into(),
            reasoning_effort: None,
            override_model: None,
        }
    }

    #[tokio::test]
    async fn per_request_model_override_is_used() {
        let provider = OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
            base_url: "http://127.0.0.1:1/v1".into(),
            api_key: "test".into(),
            model: "default-model".into(),
            reasoning_effort: None,
            max_tokens: None,
        });
        let request = LlmRequest {
            session_id: Uuid::new_v4(),
            seat: SeatKind::Mouyuan,
            phase: SessionPhase::IndependentDeliberation,
            messages: vec![],
            repair_json: false,
            max_tokens: 800,
            prompt_version: "test".into(),
            reasoning_effort: None,
            override_model: Some("override-model".into()),
        };
        // The request will fail due to connection refused, that's expected
        let result = provider.complete(request).await;
        assert!(result.is_err(), "should fail with connection error");
    }

    #[tokio::test]
    async fn seat_routed_provider_uses_seat_override() {
        let provider = SeatRoutedProvider::new(Arc::new(StaticProvider("default")))
            .with_seat_provider(SeatKind::Jingshi, Arc::new(StaticProvider("jingshi")));

        let mouyuan = provider
            .complete(request_for(SeatKind::Mouyuan))
            .await
            .unwrap();
        let jingshi = provider
            .complete(request_for(SeatKind::Jingshi))
            .await
            .unwrap();

        assert_eq!(mouyuan.content, "default");
        assert_eq!(jingshi.content, "jingshi");
    }

    #[tokio::test]
    async fn mock_provider_returns_structured_json() {
        let provider = MockProvider::new(MockScenario::SuccessMajority);
        let response = provider
            .complete(LlmRequest {
                session_id: Uuid::new_v4(),
                seat: SeatKind::Mouyuan,
                phase: SessionPhase::IndependentDeliberation,
                messages: vec![],
                repair_json: false,
                max_tokens: 800,
                prompt_version: "test".into(),
                reasoning_effort: None,
                override_model: None,
            })
            .await
            .unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&response.content).is_ok());
    }

    #[tokio::test]
    async fn mock_provider_can_return_malformed_then_repair() {
        let provider = MockProvider::new(MockScenario::MalformedThenRepair);
        let first = provider
            .complete(LlmRequest {
                session_id: Uuid::new_v4(),
                seat: SeatKind::Mouyuan,
                phase: SessionPhase::IndependentDeliberation,
                messages: vec![],
                repair_json: false,
                max_tokens: 800,
                prompt_version: "test".into(),
                reasoning_effort: None,
                override_model: None,
            })
            .await
            .unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&first.content).is_err());
        let repaired = provider
            .complete(LlmRequest {
                session_id: Uuid::new_v4(),
                seat: SeatKind::Mouyuan,
                phase: SessionPhase::IndependentDeliberation,
                messages: vec![],
                repair_json: true,
                max_tokens: 800,
                prompt_version: "test".into(),
                reasoning_effort: None,
                override_model: None,
            })
            .await
            .unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&repaired.content).is_ok());
    }
}
