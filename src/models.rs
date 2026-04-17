use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Question {
    pub id: u32,
    pub text_original: String,
    pub text_translation: String,
    pub answers: Vec<Answer>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Answer {
    pub answer_text_original: String,
    #[serde(default)]
    pub answer_text_translation: String,
    pub correct: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileInitRequest {
    pub profile_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileInitResponse {
    pub profile_id: String,
    pub memory_summary: MemorySummary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiStatusResponse {
    pub enabled: bool,
    pub model: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemorySummary {
    pub frequent_wrong_questions: Vec<u32>,
    pub concept_confusions: Vec<String>,
    pub vocabulary_weaknesses: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartExamResponse {
    pub exam_id: String,
    pub profile_id: String,
    pub total_questions: usize,
    pub passing_score: usize,
    pub dataset_correct_answers: usize,
    pub total_dataset_questions: usize,
    pub questions: Vec<ExamQuestionPayload>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExamQuestionPayload {
    pub id: u32,
    pub text_original: String,
    pub text_translation: String,
    pub answers: Vec<AnswerOptionPayload>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnswerOptionPayload {
    pub key: String,
    pub text_original: String,
    pub text_translation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HintRequest {
    pub exam_id: String,
    pub question_id: u32,
    pub profile_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranslationRequest {
    pub exam_id: String,
    pub question_id: u32,
    pub profile_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranslationResponse {
    pub question_id: u32,
    pub generated_answers: Vec<GeneratedAnswerTranslation>,
    pub source: Option<String>,
    pub ai_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneratedAnswerTranslation {
    pub key: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitAnswerRequest {
    pub exam_id: String,
    pub question_id: u32,
    pub selected_key: String,
    pub profile_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HintResponse {
    pub question_id: u32,
    pub hint: Option<AiHintResponse>,
    pub source: Option<String>,
    pub ai_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitAnswerResponse {
    pub exam_id: String,
    pub question_id: u32,
    pub selected_key: String,
    pub correct_key: String,
    pub is_correct: bool,
    pub correct_answer_text_original: String,
    pub feedback: Option<AiHintResponse>,
    pub feedback_source: Option<String>,
    pub ai_error: Option<String>,
    pub score: usize,
    pub answered_count: usize,
    pub total_questions: usize,
    pub dataset_correct_answers: usize,
    pub total_dataset_questions: usize,
    pub is_exam_complete: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExamResultResponse {
    pub exam_id: String,
    pub profile_id: String,
    pub score: usize,
    pub total_questions: usize,
    pub passing_score: usize,
    pub passed: bool,
    pub answered_count: usize,
    pub dataset_correct_answers: usize,
    pub total_dataset_questions: usize,
    pub incorrect_questions: Vec<ReviewItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReviewResponse {
    pub exam_id: String,
    pub profile_id: String,
    pub correct_questions: Vec<ReviewItem>,
    pub incorrect_questions: Vec<ReviewItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileReviewResponse {
    pub profile_id: String,
    pub correct_questions: Vec<ReviewItem>,
    pub incorrect_questions: Vec<ReviewItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReviewItem {
    pub question_id: u32,
    pub text_original: String,
    pub text_translation: String,
    pub selected_key: String,
    pub selected_answer_text_original: String,
    pub correct_key: String,
    pub correct_answer_text_original: String,
    pub is_correct: bool,
    pub feedback_text: String,
    pub memory_trick: String,
    pub ai_error: Option<String>,
    pub submitted_at: DateTime<Utc>,
    pub answers: Vec<AnswerOptionPayload>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExamRecord {
    pub exam_id: String,
    pub profile_id: String,
    pub question_ids: Vec<u32>,
    pub submissions: Vec<SubmittedAnswer>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmittedAnswer {
    pub question_id: u32,
    pub selected_key: String,
    pub correct_key: String,
    pub is_correct: bool,
    pub feedback: Option<AiHintResponse>,
    pub ai_error: Option<String>,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LearnerMemory {
    pub profile_id: String,
    #[serde(default)]
    pub correctly_answered_question_ids: Vec<u32>,
    #[serde(default)]
    pub frequently_wrong_question_ids: HashMap<u32, u32>,
    #[serde(default)]
    pub concept_confusions: HashMap<String, u32>,
    #[serde(default)]
    pub vocabulary_weaknesses: HashMap<String, u32>,
    #[serde(default)]
    pub recovery_streaks: HashMap<u32, u32>,
    #[serde(default)]
    pub question_cooldowns: HashMap<u32, u32>,
    #[serde(default)]
    pub recent_sessions: Vec<RecentSession>,
}

impl LearnerMemory {
    pub fn empty(profile_id: String) -> Self {
        Self {
            profile_id,
            correctly_answered_question_ids: Vec::new(),
            frequently_wrong_question_ids: HashMap::new(),
            concept_confusions: HashMap::new(),
            vocabulary_weaknesses: HashMap::new(),
            recovery_streaks: HashMap::new(),
            question_cooldowns: HashMap::new(),
            recent_sessions: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecentSession {
    pub exam_id: String,
    pub score: usize,
    pub total_questions: usize,
    pub completed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiHintResponse {
    #[serde(rename = "emojiQuestion")]
    pub emoji_question: String,
    #[serde(rename = "emojiOptions")]
    pub emoji_options: Vec<EmojiOption>,
    #[serde(rename = "hintTitle")]
    pub hint_title: String,
    #[serde(rename = "hintText")]
    pub hint_text: String,
    #[serde(rename = "feedbackText")]
    pub feedback_text: String,
    #[serde(rename = "memoryTrick")]
    pub memory_trick: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmojiOption {
    pub key: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiRequestContext {
    pub question_id: u32,
    pub question_text_original: String,
    pub question_text_translation: String,
    pub answers: Vec<AiAnswerContext>,
    pub correct_key: String,
    pub selected_key: Option<String>,
    pub is_correct: Option<bool>,
    pub memory_summary: MemorySummary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiAnswerContext {
    pub key: String,
    pub answer_text_original: String,
    pub answer_text_translation: String,
}
