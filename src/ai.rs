use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::models::{AiHintResponse, AiRequestContext, GeneratedAnswerTranslation, Question};

pub const DEFAULT_OPENAI_MODEL: &str = "gpt-5.4";

const SYSTEM_PROMPT: &str = r#"You are a concise tutor for beginners preparing for the German Einbuergerungstest.

Rules:
- You are not the judge of correctness.
- Correctness is already decided by the dataset and provided as fact.
- The quiz itself stays in German.
- Explanations must be short, beginner-friendly, and mostly English with important German words preserved.
- Use simple mappings like "Bundesland = state", "waehlen = vote", "Polizei = Sicherheit".
- Emoji must be concrete and mnemonic, not decorative.
- Prefer real-world object/action emoji that match the meaning:
  - Polizei -> 🚓 or 👮
  - Parlament -> 🏛️
  - Wahl / waehlen -> 🗳️
  - Schule -> 🏫
  - Gericht -> ⚖️
  - Arbeit / Firma -> 💼 or 🏢
- Avoid generic emoji like 👉, ✨, or random faces unless they clearly help memory.
- For each answer option, pick emoji that distinguish that option from the others.
- Avoid advanced grammar explanations.
- Keep memory tricks short and practical.
- Always return valid JSON matching the requested schema.
"#;

#[derive(Clone)]
pub struct AiClient {
    http: Client,
    api_key: Option<String>,
    model: String,
}

impl AiClient {
    pub fn new(api_key: Option<String>, model: Option<String>) -> Self {
        Self {
            http: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| DEFAULT_OPENAI_MODEL.to_string()),
        }
    }

    pub async fn generate_hint(&self, context: &AiRequestContext) -> Result<(AiHintResponse, String)> {
        if self.api_key.is_none() {
            return Err(anyhow!("missing OPENAI_API_KEY"));
        }

        let hint = self.call_openai(context).await?;
        Ok((hint, "openai".to_string()))
    }

    pub async fn generate_missing_answer_translations(
        &self,
        question: &Question,
    ) -> Result<(Vec<GeneratedAnswerTranslation>, String)> {
        let api_key = self.api_key.clone().ok_or_else(|| anyhow!("missing OPENAI_API_KEY"))?;
        let request_body = json!({
            "model": self.model,
            "input": [
                {
                    "role": "system",
                    "content": [
                        {
                            "type": "input_text",
                            "text": "You translate German citizenship-test answers into short, clear English. Return JSON only. Keep it concise and literal. Do not explain. Translate only the answer options that are missing dataset translations."
                        }
                    ]
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "input_text",
                            "text": build_translation_prompt(question)
                        }
                    ]
                }
            ],
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": "missing_answer_translations",
                    "schema": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "generated_answers": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "additionalProperties": false,
                                    "properties": {
                                        "key": { "type": "string" },
                                        "text": { "type": "string" }
                                    },
                                    "required": ["key", "text"]
                                }
                            }
                        },
                        "required": ["generated_answers"]
                    }
                }
            }
        });

        let response = self
            .http
            .post("https://api.openai.com/v1/responses")
            .bearer_auth(api_key)
            .json(&request_body)
            .send()
            .await
            .context("failed to call OpenAI Responses API")?
            .error_for_status()
            .context("OpenAI returned an error")?;

        let payload: OpenAiResponsesPayload = response.json().await?;
        let text = payload
            .output_text()
            .ok_or_else(|| anyhow!("OpenAI response did not include output_text"))?;

        let parsed: GeneratedTranslationsPayload =
            serde_json::from_str(&text).context("failed to parse generated translations JSON")?;
        Ok((parsed.generated_answers, "openai".to_string()))
    }

    async fn call_openai(&self, context: &AiRequestContext) -> Result<AiHintResponse> {
        let api_key = self.api_key.clone().ok_or_else(|| anyhow!("missing OPENAI_API_KEY"))?;
        let user_prompt = build_user_prompt(context);

        let request_body = json!({
            "model": self.model,
            "input": [
                {
                    "role": "system",
                    "content": [
                        {
                            "type": "input_text",
                            "text": SYSTEM_PROMPT
                        }
                    ]
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "input_text",
                            "text": user_prompt
                        }
                    ]
                }
            ],
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": "einbuergerung_hint",
                    "schema": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "emojiQuestion": { "type": "string" },
                            "emojiOptions": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "additionalProperties": false,
                                    "properties": {
                                        "key": { "type": "string" },
                                        "text": { "type": "string" }
                                    },
                                    "required": ["key", "text"]
                                }
                            },
                            "hintTitle": { "type": "string" },
                            "hintText": { "type": "string" },
                            "feedbackText": { "type": "string" },
                            "memoryTrick": { "type": "string" }
                        },
                        "required": [
                            "emojiQuestion",
                            "emojiOptions",
                            "hintTitle",
                            "hintText",
                            "feedbackText",
                            "memoryTrick"
                        ]
                    }
                }
            }
        });

        let response = self
            .http
            .post("https://api.openai.com/v1/responses")
            .bearer_auth(api_key)
            .json(&request_body)
            .send()
            .await
            .context("failed to call OpenAI Responses API")?
            .error_for_status()
            .context("OpenAI returned an error")?;

        let payload: OpenAiResponsesPayload = response.json().await?;
        let text = payload
            .output_text()
            .ok_or_else(|| anyhow!("OpenAI response did not include output_text"))?;

        serde_json::from_str(&text).context("failed to parse OpenAI JSON hint")
    }

    pub fn is_enabled(&self) -> bool {
        self.api_key.is_some()
    }

    pub fn model_name(&self) -> Option<&str> {
        self.api_key.as_ref().map(|_| self.model.as_str())
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiResponsesPayload {
    output: Vec<OpenAiOutput>,
}

impl OpenAiResponsesPayload {
    fn output_text(&self) -> Option<String> {
        let mut parts = Vec::new();
        for output in &self.output {
            for content in &output.content {
                if let Some(text) = &content.text {
                    parts.push(text.clone());
                }
            }
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n"))
        }
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiOutput {
    content: Vec<OpenAiContent>,
}

#[derive(Debug, Deserialize)]
struct OpenAiContent {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeneratedTranslationsPayload {
    generated_answers: Vec<GeneratedAnswerTranslation>,
}

#[derive(Debug, Serialize)]
pub struct PromptSamples {
    pub system_prompt: &'static str,
    pub user_prompt_template: String,
}

pub fn prompt_samples() -> PromptSamples {
    let example = AiRequestContext {
        question_id: 286,
        question_text_original: "Welche Organisation in einer Firma hilft den Arbeitnehmern und Arbeitnehmerinnen bei Problemen mit dem Arbeitgeber / der Arbeitgeberin?".to_string(),
        question_text_translation: "Which organization in a company helps employees with problems with the employer?".to_string(),
        answers: vec![
            ("A", "der Betriebsrat", "the works council"),
            ("B", "der Betriebspruefer / die Betriebsprueferin", "the auditor"),
            ("C", "die Betriebsgruppe", "the work group"),
            ("D", "das Betriebsmanagement", "the operations management"),
        ]
        .into_iter()
        .map(|(key, original, translation)| crate::models::AiAnswerContext {
            key: key.to_string(),
            answer_text_original: original.to_string(),
            answer_text_translation: translation.to_string(),
        })
        .collect(),
        correct_key: "A".to_string(),
        selected_key: Some("B".to_string()),
        is_correct: Some(false),
        memory_summary: crate::models::MemorySummary {
            frequent_wrong_questions: vec![286],
            concept_confusions: vec!["B -> A".to_string()],
            vocabulary_weaknesses: vec!["firma".to_string(), "probleme".to_string()],
        },
    };

    PromptSamples {
        system_prompt: SYSTEM_PROMPT,
        user_prompt_template: build_user_prompt(&example),
    }
}

fn build_user_prompt(context: &AiRequestContext) -> String {
    let selected = context.selected_key.clone().unwrap_or_else(|| "none".to_string());
    let correctness = context
        .is_correct
        .map(|value| if value { "correct" } else { "incorrect" })
        .unwrap_or("not_answered");

    format!(
        "Return JSON only.\n\nQuestion id: {}\nGerman question: {}\nEnglish question: {}\nAnswers: {}\nCorrect answer key: {}\nSelected answer key: {}\nResult status: {}\nMemory summary: frequent wrong question ids = {:?}; concept confusions = {:?}; vocabulary weaknesses = {:?}\n\nRequirements:\n- emojiQuestion should add a few helpful emoji but keep the full German question.\n- emojiQuestion should use emoji that match the key nouns or actions in the meaning, not generic decoration.\n- emojiOptions should keep the answer keys and German answers.\n- emojiOptions should use distinct, concrete emoji that help a beginner guess the meaning from context.\n- Good examples: Polizei -> 🚓, Parlament -> 🏛️, Wahl -> 🗳️, Schule -> 🏫, Gericht -> ⚖️.\n- Bad example: adding the same generic emoji to every option.\n- hintTitle should be short.\n- hintText should be concise and use easy English + some German words.\n- feedbackText should explain the answer using the provided result status as fact.\n- memoryTrick should be short and memorable.\n- Keep the tone clear and encouraging.\n- Do not say you are uncertain about correctness.",
        context.question_id,
        context.question_text_original,
        context.question_text_translation,
        context
            .answers
            .iter()
            .map(|answer| format!(
                "{}: {} ({})",
                answer.key, answer.answer_text_original, answer.answer_text_translation
            ))
            .collect::<Vec<_>>()
            .join("; "),
        context.correct_key,
        selected,
        correctness,
        context.memory_summary.frequent_wrong_questions,
        context.memory_summary.concept_confusions,
        context.memory_summary.vocabulary_weaknesses
    )
}

fn build_translation_prompt(question: &Question) -> String {
    let missing_answers = question
        .answers
        .iter()
        .enumerate()
        .filter(|(_, answer)| answer.answer_text_translation.trim().is_empty())
        .map(|(index, answer)| format!("{}: {}", crate::quiz::answer_key(index), answer.answer_text_original))
        .collect::<Vec<_>>()
        .join("; ");

    format!(
        "Question id: {}\nGerman question: {}\nEnglish question translation from dataset: {}\nTranslate only these missing answer options into simple English: {}",
        question.id,
        question.text_original,
        question.text_translation,
        missing_answers
    )
}
