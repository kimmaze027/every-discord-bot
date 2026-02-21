use crate::ai::db::ChatMessage;
use base64::Engine;
use serde::{Deserialize, Serialize};

const API_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent";

const SYSTEM_PROMPT: &str = "\
당신은 EveryBot, Escape from Tarkov 전문 디스코드 봇입니다.
모든 대화를 Escape from Tarkov 게임 맥락에서 해석하고 답변하세요.

아이템 관련 질문 시:
- tarkov.dev API 검색 결과가 함께 제공되면, 그 실시간 데이터를 기반으로 정확한 가격과 정보를 알려주세요.
- 검색 결과에 여러 아이템이 있으면, 각각의 차이점과 가격을 비교해서 알려주세요.
- API 데이터가 없으면 알고 있는 일반적인 정보로 답변하되, 정확한 가격은 확인이 필요하다고 안내하세요.

전략, 맵, 퀘스트, 탄약, 무기, 보스, 은신처 등 타르코프 관련 지식을 활용하세요.
최근 채팅 맥락을 참고하여 자연스럽게 대화하세요.
한국어로 응답하세요.";

const IMAGE_SYSTEM_PROMPT: &str = "\
당신은 EveryBot, Escape from Tarkov 전문 디스코드 봇입니다.
사용자가 이미지를 보냈습니다. 타르코프 관련 이미지(인벤토리, 전리품, 맵, 탄약표 등)라면 아이템을 식별하고 대략적인 가치와 활용법을 분석해주세요.
타르코프와 무관한 이미지라도 가능하면 타르코프에 연관지어 재미있게 설명해주세요.
한국어로 응답하세요.";

#[derive(Debug)]
pub enum GeminiError {
    Http(reqwest::Error),
    Api(String),
}

impl std::fmt::Display for GeminiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(e) => write!(f, "HTTP error: {e}"),
            Self::Api(e) => write!(f, "API error: {e}"),
        }
    }
}

impl std::error::Error for GeminiError {}

impl From<reqwest::Error> for GeminiError {
    fn from(e: reqwest::Error) -> Self {
        Self::Http(e)
    }
}

// --- Request types ---

#[derive(Serialize)]
struct Request {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<Content>,
    contents: Vec<Content>,
}

#[derive(Serialize)]
struct Content {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<Part>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Part {
    Text { text: String },
    InlineData { inline_data: InlineData },
}

#[derive(Serialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

// --- Response types ---

#[derive(Deserialize)]
struct Response {
    candidates: Option<Vec<Candidate>>,
    error: Option<ApiError>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Option<Vec<CandidatePart>>,
}

#[derive(Deserialize)]
struct CandidatePart {
    text: Option<String>,
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
}

fn extract_text(response: Response) -> Result<Option<String>, GeminiError> {
    if let Some(err) = response.error {
        return Err(GeminiError::Api(err.message));
    }

    Ok(response
        .candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .and_then(|p| p.into_iter().next())
        .and_then(|p| p.text))
}

/// DB에서 가져온 최근 대화를 기반으로 Gemini에 채팅 요청.
/// 멘션 시에만 호출되므로 항상 응답.
/// `tarkov_context`: tarkov.dev API 검색 결과 (있으면 시스템 프롬프트에 추가)
pub async fn chat(
    client: &reqwest::Client,
    api_key: &str,
    messages: &[ChatMessage],
    tarkov_context: Option<&str>,
) -> Result<String, GeminiError> {
    if messages.is_empty() {
        return Err(GeminiError::Api("메시지 없음".to_string()));
    }

    // 메시지를 Gemini contents로 변환 (연속 같은 role 병합)
    let mut contents: Vec<Content> = Vec::new();

    for msg in messages {
        let role = if msg.is_bot { "model" } else { "user" };
        let text = if msg.is_bot {
            msg.content.clone()
        } else {
            format!("[{}] {}", msg.author_name, msg.content)
        };

        if let Some(last) = contents.last_mut() {
            if last.role.as_deref() == Some(role) {
                last.parts.push(Part::Text { text });
                continue;
            }
        }

        contents.push(Content {
            role: Some(role.to_string()),
            parts: vec![Part::Text { text }],
        });
    }

    // 첫 메시지가 model이면 user가 나올 때까지 건너뛰기 (API 요구사항)
    let start = match contents
        .iter()
        .position(|c| c.role.as_deref() == Some("user"))
    {
        Some(idx) => idx,
        None => return Err(GeminiError::Api("user 메시지 없음".to_string())),
    };
    let contents: Vec<Content> = contents.into_iter().skip(start).collect();

    let system_text = match tarkov_context {
        Some(ctx) => format!("{SYSTEM_PROMPT}{ctx}"),
        None => SYSTEM_PROMPT.to_string(),
    };

    let request = Request {
        system_instruction: Some(Content {
            role: None,
            parts: vec![Part::Text { text: system_text }],
        }),
        contents,
    };

    let resp = client
        .post(format!("{API_URL}?key={api_key}"))
        .json(&request)
        .send()
        .await?;

    let response: Response = resp.json().await?;

    extract_text(response)?.ok_or_else(|| GeminiError::Api("빈 응답".to_string()))
}

/// 이미지를 분석하여 설명을 반환. 항상 응답 (SKIP 불가).
pub async fn analyze_image(
    client: &reqwest::Client,
    api_key: &str,
    image_bytes: &[u8],
    mime_type: &str,
    user_text: Option<&str>,
) -> Result<String, GeminiError> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(image_bytes);

    let mut parts = vec![Part::InlineData {
        inline_data: InlineData {
            mime_type: mime_type.to_string(),
            data: b64,
        },
    }];

    if let Some(text) = user_text {
        parts.push(Part::Text {
            text: text.to_string(),
        });
    }

    let request = Request {
        system_instruction: Some(Content {
            role: None,
            parts: vec![Part::Text {
                text: IMAGE_SYSTEM_PROMPT.to_string(),
            }],
        }),
        contents: vec![Content {
            role: Some("user".to_string()),
            parts,
        }],
    };

    let resp = client
        .post(format!("{API_URL}?key={api_key}"))
        .json(&request)
        .send()
        .await?;

    let response: Response = resp.json().await?;

    extract_text(response)?.ok_or_else(|| GeminiError::Api("빈 응답".to_string()))
}
