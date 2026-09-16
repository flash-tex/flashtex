//! OpenAI-compatible Chat Completions provider (`POST {base}/chat/completions`).
//!
//! Lets `capture_convert` use any vision-capable endpoint that speaks this
//! de-facto wire format — a hosted API or a local model server on loopback —
//! with the same prompt, strict proposal schema, validation, retry bounds and
//! review gate as the xAI path. Nothing is sent until an explicit conversion.
use crate::{
    grok::{self, Transport},
    provider::ProviderEvidence,
    BridgeError, CaptureSubmit, Context, Converted, Converter, Proposal, Result, MAX_LATEX_BYTES,
};
use serde_json::{json, Value};

pub const PROVIDER_NAME: &str = "openai-compatible";

pub fn request_body(model: &str, capture: &CaptureSubmit, context: &Context) -> Value {
    json!({
        "model": model,
        "stream": false,
        "messages": [
            {"role": "system", "content": grok::SYSTEM_PROMPT},
            {"role": "user", "content": [
                {"type": "text", "text": grok::user_text(capture, context)},
                {"type": "image_url", "image_url": {"url": grok::image_data_url(capture), "detail": "high"}}
            ]}
        ],
        "response_format": {"type": "json_schema", "json_schema": {
            "name": "latex_capture_proposal", "strict": true, "schema": grok::proposal_schema()
        }}
    })
}

/// Parses one Chat Completions reply into a validated proposal.
pub fn parse_response(value: &Value) -> Result<Proposal> {
    let invalid = |message: &str| BridgeError::new("provider_invalid_response", message);
    let choices = value
        .get("choices")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("Missing choices array"))?;
    if choices.len() != 1 {
        return Err(invalid("Expected exactly one choice"));
    }
    let choice = &choices[0];
    match choice.get("finish_reason").and_then(Value::as_str) {
        None | Some("stop") => {}
        Some("content_filter") => {
            return Err(BridgeError::new(
                "provider_refusal",
                "The conversion provider declined this conversion",
            ))
        }
        Some(_) => {
            return Err(BridgeError::new(
                "provider_incomplete",
                "The conversion provider did not finish its response; no insertion is available",
            ))
        }
    }
    let message = choice
        .get("message")
        .filter(|m| m.is_object())
        .ok_or_else(|| invalid("Missing message"))?;
    if message
        .get("refusal")
        .and_then(Value::as_str)
        .is_some_and(|r| !r.is_empty())
    {
        return Err(BridgeError::new(
            "provider_refusal",
            "The conversion provider declined this conversion",
        ));
    }
    let text = match message.get("content") {
        Some(Value::String(text)) => text.clone(),
        // Some servers return content parts; accept exactly one text part.
        Some(Value::Array(parts)) => {
            let texts: Vec<&str> = parts
                .iter()
                .filter(|p| p.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|p| p.get("text").and_then(Value::as_str))
                .collect();
            if texts.len() != 1 {
                return Err(invalid("Expected one bounded structured proposal"));
            }
            texts[0].to_string()
        }
        _ => return Err(invalid("Message content is not text")),
    };
    if text.len() > MAX_LATEX_BYTES + 128 * 1024 {
        return Err(invalid("Expected one bounded structured proposal"));
    }
    let proposal: Proposal = serde_json::from_str(strip_json_fence(&text))
        .map_err(|_| invalid("Conversion proposal did not match the required JSON shape"))?;
    proposal.validate()?;
    Ok(proposal)
}

/// Servers without structured-output support often wrap JSON in a Markdown
/// fence. Only a whole-reply fence is removed; the schema check still applies.
fn strip_json_fence(text: &str) -> &str {
    let trimmed = text.trim();
    trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|rest| rest.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(trimmed)
}

pub fn evidence(value: &Value, configured_model: &str) -> ProviderEvidence {
    ProviderEvidence::from_reply(
        PROVIDER_NAME,
        configured_model,
        value,
        ("prompt_tokens", "completion_tokens", "total_tokens"),
    )
}

pub struct OpenAiCompatibleClient {
    key: Option<String>,
    model: String,
    endpoint: String,
    transport: Transport,
}

impl OpenAiCompatibleClient {
    /// `base_url` must already have passed `provider::validate_base_url`.
    /// A missing key sends no `Authorization` header (loopback servers only;
    /// `ProviderConfig::resolve` enforces that).
    pub fn new(key: Option<String>, model: String, base_url: &str) -> Result<Self> {
        if !grok::valid_model(&model) {
            return Err(BridgeError::new(
                "invalid_model",
                "Configure a valid conversion model ID",
            ));
        }
        Ok(Self {
            key: key.filter(|k| !k.trim().is_empty()),
            model,
            endpoint: format!("{}/chat/completions", base_url.trim_end_matches('/')),
            transport: Transport::new("The conversion provider")?,
        })
    }
}

impl Converter for OpenAiCompatibleClient {
    fn convert(&self, capture: &CaptureSubmit, context: &Context) -> Result<Proposal> {
        self.convert_with_evidence(capture, context)
            .map(|c| c.proposal)
    }
    fn convert_with_evidence(
        &self,
        capture: &CaptureSubmit,
        context: &Context,
    ) -> Result<Converted> {
        let body = request_body(&self.model, capture, context);
        self.transport
            .post_json(&self.endpoint, self.key.as_deref(), &body, |value| {
                Ok(Converted {
                    proposal: parse_response(value)?,
                    evidence: Some(evidence(value, &self.model)),
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CaptureImage;

    fn completion(content: Value, finish: &str) -> Value {
        json!({
            "id": "chatcmpl-fixture",
            "model": "vision-fixture",
            "choices": [{"index": 0, "finish_reason": finish, "message": {"role": "assistant", "content": content}}],
            "usage": {"prompt_tokens": 120, "completion_tokens": 30, "total_tokens": 150}
        })
    }
    fn proposal_text() -> String {
        json!({"latex":"$x^2$","ambiguities":[],"required_dependencies":[]}).to_string()
    }

    #[test]
    fn request_uses_chat_completions_shape_with_strict_schema_and_image() {
        let capture = CaptureSubmit {
            capture_id: "cap".into(),
            destination_id: "dest".into(),
            base_revision: 1,
            image: CaptureImage {
                mime_type: "image/png".into(),
                data_base64: "AA==".into(),
            },
            instructions: "transcribe".into(),
        };
        let context = Context {
            caret_context: Default::default(),
            project_id: "p".into(),
            path: "main.tex".into(),
            revision: 1,
            source_before: String::new(),
            selected_source: String::new(),
            source_after: String::new(),
            definitions: vec![],
            supported_features: vec!["\\frac{}{}".into()],
            dependencies: vec![],
        };
        let body = request_body("vision-fixture", &capture, &context);
        assert_eq!(body["model"], "vision-fixture");
        assert_eq!(body["messages"][0]["content"], grok::SYSTEM_PROMPT);
        assert_eq!(body["response_format"]["json_schema"]["strict"], true);
        assert_eq!(
            body["response_format"]["json_schema"]["schema"],
            grok::proposal_schema()
        );
        assert_eq!(
            body["messages"][1]["content"][1]["image_url"]["url"],
            "data:image/png;base64,AA=="
        );
        let text = body["messages"][1]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\\\\frac{}{}"), "{text}");
        assert!(body.get("store").is_none());
    }

    #[test]
    fn happy_path_and_evidence() {
        let reply = completion(Value::String(proposal_text()), "stop");
        assert_eq!(parse_response(&reply).unwrap().latex, "$x^2$");
        let e = evidence(&reply, "configured");
        assert_eq!(e.provider, "openai-compatible");
        assert_eq!(e.model, "vision-fixture");
        assert_eq!(e.response_id.as_deref(), Some("chatcmpl-fixture"));
        let usage = e.usage.unwrap();
        assert_eq!(
            (usage.input_tokens, usage.output_tokens, usage.total_tokens),
            (Some(120), Some(30), Some(150))
        );
    }

    #[test]
    fn content_parts_and_whole_reply_fences_are_accepted() {
        let parts = completion(json!([{"type":"text","text":proposal_text()}]), "stop");
        assert!(parse_response(&parts).is_ok());
        let fenced = completion(
            Value::String(format!("```json\n{}\n```", proposal_text())),
            "stop",
        );
        assert!(parse_response(&fenced).is_ok());
    }

    #[test]
    fn truncation_refusal_and_filter_are_distinct_errors() {
        let cut = completion(Value::String(proposal_text()), "length");
        assert_eq!(
            parse_response(&cut).unwrap_err().code,
            "provider_incomplete"
        );
        let filtered = completion(Value::Null, "content_filter");
        assert_eq!(
            parse_response(&filtered).unwrap_err().code,
            "provider_refusal"
        );
        let refusal =
            json!({"choices":[{"finish_reason":"stop","message":{"content":null,"refusal":"no"}}]});
        assert_eq!(
            parse_response(&refusal).unwrap_err().code,
            "provider_refusal"
        );
    }

    #[test]
    fn malformed_replies_are_invalid_responses() {
        for reply in [
            json!({}),
            json!({"choices": []}),
            json!({"choices": [{"finish_reason":"stop","message":{"content":"a"}}, {"finish_reason":"stop","message":{"content":"b"}}]}),
            json!({"choices": [{"finish_reason":"stop"}]}),
            completion(json!(42), "stop"),
            completion(Value::String("not json".into()), "stop"),
            completion(
                Value::String(
                    json!({"latex":"$x$","ambiguities":[],"required_dependencies":[],"extra":1})
                        .to_string(),
                ),
                "stop",
            ),
        ] {
            assert_eq!(
                parse_response(&reply).unwrap_err().code,
                "provider_invalid_response",
                "{reply}"
            );
        }
    }

    #[test]
    fn unsafe_latex_fails_the_shared_proposal_validation() {
        let reply = completion(
            Value::String(
                json!({"latex":"\\write18{rm -rf /}","ambiguities":[],"required_dependencies":[]})
                    .to_string(),
            ),
            "stop",
        );
        assert_ne!(
            parse_response(&reply).unwrap_err().code,
            "provider_invalid_response"
        );
        assert!(parse_response(&reply).is_err());
    }
}
