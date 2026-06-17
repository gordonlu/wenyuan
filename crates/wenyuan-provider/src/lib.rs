use async_trait::async_trait;
use serde::{Deserialize, Serialize};
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
    pub temperature: f32,
    pub max_tokens: u32,
    pub prompt_version: String,
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
    #[error("provider request failed: {0}")]
    Request(String),
    #[error("provider returned invalid response: {0}")]
    InvalidResponse(String),
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, ProviderError>;
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
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
    temperature: f32,
    max_tokens: u32,
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
        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );
        let body = OpenAiRequest {
            model: &self.config.model,
            messages: &request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
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
            return Err(ProviderError::Request(format!("http status {status}")));
        }
        let payload = response
            .json::<OpenAiResponse>()
            .await
            .map_err(|err| ProviderError::InvalidResponse(err.to_string()))?;
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
            "assumptions": ["用户愿意提供足够背景"],
            "risks": ["模型输出需要格式校验"]
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
                "suggested_improvement": "补充验收指标和失败边界"
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
        "user_value": "用结构化过程替代松散聊天",
        "implementation_path": "先 Mock 跑通，再接真实 Provider",
        "risks": ["真实模型格式稳定性", "阶段并发控制"],
        "success_metrics": ["完整流程可重复执行", "多数和少数意见可追溯"]
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
                "confidence": 0.82
            }
        ]
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
                temperature: 0.7,
                max_tokens: 800,
                prompt_version: "test".into(),
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
                temperature: 0.7,
                max_tokens: 800,
                prompt_version: "test".into(),
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
                temperature: 0.7,
                max_tokens: 800,
                prompt_version: "test".into(),
            })
            .await
            .unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&repaired.content).is_ok());
    }
}
