use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use rand::Rng;

use crate::ai::{AiClient, prompt_samples};
use crate::memory::{
    advance_question_cooldowns, cooldown_question_ids, dataset_correct_answers, load_memory,
    prioritized_question_ids, record_completed_session, save_memory, summarize,
    update_after_answer,
};
use crate::models::{
    AiStatusResponse, ExamRecord, ExamResultResponse, HintRequest, HintResponse, ProfileInitRequest,
    ProfileInitResponse, ProfileReviewResponse, Question, RecentSession, ReviewResponse,
    StartExamResponse, SubmitAnswerRequest, SubmitAnswerResponse, SubmittedAnswer,
    TranslationRequest, TranslationResponse,
};
use crate::quiz::{
    PASSING_SCORE, answer_index, build_ai_context, correct_answer, create_exam, review_items,
    review_items_for_records, review_items_split, upsert_submission,
};
use crate::storage::{read_json_file, root_data_dir, write_json_file};

#[derive(Clone)]
pub struct AppState {
    pub questions: Arc<Vec<Question>>,
    pub exams_dir: Arc<PathBuf>,
    pub memory_dir: Arc<PathBuf>,
    pub ai_client: AiClient,
}

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/api/profile/init", post(init_profile))
        .route("/api/ai/status", get(get_ai_status))
        .route("/api/exam/start", get(start_exam))
        .route("/api/exam/hint", post(get_hint))
        .route("/api/exam/translation", post(get_translation))
        .route("/api/exam/answer", post(submit_answer))
        .route("/api/exam/result/{id}", get(get_result))
        .route("/api/exam/review/{id}", get(get_review))
        .route("/api/profile/review", get(get_profile_review))
        .route("/api/meta/prompts", get(get_prompt_samples))
        .with_state(state)
}

pub async fn load_questions() -> Result<Vec<Question>> {
    let default_paths = [
        root_data_dir().join("questions.json"),
        PathBuf::from("questions-answers-dataset.json"),
    ];

    for path in default_paths {
        if tokio::fs::metadata(&path).await.is_ok() {
            return read_json_file(&path).await;
        }
    }

    Err(anyhow!("could not find questions JSON file"))
}

async fn init_profile(
    State(state): State<AppState>,
    Json(payload): Json<ProfileInitRequest>,
) -> Result<Json<ProfileInitResponse>, ApiError> {
    let profile_id = payload
        .profile_id
        .as_deref()
        .map(sanitize_profile_id)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(generate_profile_id);

    let memory = load_memory(&state.memory_dir, &profile_id)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(ProfileInitResponse {
        profile_id,
        memory_summary: summarize(&memory),
    }))
}

async fn start_exam(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<StartExamResponse>, ApiError> {
    let profile_id = params
        .get("profile_id")
        .map(|value| sanitize_profile_id(value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(generate_profile_id);

    let mut memory = load_memory(&state.memory_dir, &profile_id)
        .await
        .map_err(ApiError::internal)?;
    let prioritized_ids = prioritized_question_ids(&memory, state.questions.len());
    let excluded_ids = cooldown_question_ids(&memory);
    let dataset_correct_answers = dataset_correct_answers(&memory);

    let (record, response) = create_exam(
        profile_id,
        &state.questions,
        &prioritized_ids,
        &excluded_ids,
        dataset_correct_answers,
    );
    advance_question_cooldowns(&mut memory);
    save_memory(&state.memory_dir, &memory)
        .await
        .map_err(ApiError::internal)?;
    save_exam(&state, &record).await.map_err(ApiError::internal)?;
    Ok(Json(response))
}

async fn get_ai_status(State(state): State<AppState>) -> Json<AiStatusResponse> {
    Json(AiStatusResponse {
        enabled: state.ai_client.is_enabled(),
        model: state.ai_client.model_name().map(ToOwned::to_owned),
        status: if state.ai_client.is_enabled() {
            "enabled".to_string()
        } else {
            "disabled".to_string()
        },
    })
}

async fn get_hint(
    State(state): State<AppState>,
    Json(payload): Json<HintRequest>,
) -> Result<Json<HintResponse>, ApiError> {
    let record = load_exam(&state, &payload.exam_id).await.map_err(ApiError::internal)?;
    ensure_exam_access(&record, &payload.profile_id)?;
    ensure_question_in_exam(&record, payload.question_id)?;

    let question = question_by_id(&state.questions, payload.question_id)?;
    let memory = load_memory(&state.memory_dir, &payload.profile_id)
        .await
        .map_err(ApiError::internal)?;

    let context = build_ai_context(question, None, None, summarize(&memory));
    let (hint, source, ai_error) = match state.ai_client.generate_hint(&context).await {
        Ok((hint, source)) => (Some(hint), Some(source), None),
        Err(error) => (None, None, Some(error.to_string())),
    };

    Ok(Json(HintResponse {
        question_id: payload.question_id,
        hint,
        source,
        ai_error,
    }))
}

async fn get_translation(
    State(state): State<AppState>,
    Json(payload): Json<TranslationRequest>,
) -> Result<Json<TranslationResponse>, ApiError> {
    let record = load_exam(&state, &payload.exam_id).await.map_err(ApiError::internal)?;
    ensure_exam_access(&record, &payload.profile_id)?;
    ensure_question_in_exam(&record, payload.question_id)?;

    let question = question_by_id(&state.questions, payload.question_id)?;
    let (generated_answers, source, ai_error) = match state
        .ai_client
        .generate_missing_answer_translations(question)
        .await
    {
        Ok((generated_answers, source)) => (generated_answers, Some(source), None),
        Err(error) => (Vec::new(), None, Some(error.to_string())),
    };

    Ok(Json(TranslationResponse {
        question_id: payload.question_id,
        generated_answers,
        source,
        ai_error,
    }))
}

async fn submit_answer(
    State(state): State<AppState>,
    Json(payload): Json<SubmitAnswerRequest>,
) -> Result<Json<SubmitAnswerResponse>, ApiError> {
    let mut record = load_exam(&state, &payload.exam_id).await.map_err(ApiError::internal)?;
    ensure_exam_access(&record, &payload.profile_id)?;
    ensure_question_in_exam(&record, payload.question_id)?;

    let selected_index = answer_index(&payload.selected_key)
        .ok_or_else(|| ApiError::bad_request("selected_key must be A, B, C, or D"))?;
    let question = question_by_id(&state.questions, payload.question_id)?;
    let (correct_index, correct_key) = correct_answer(question);
    let is_correct = selected_index == correct_index;

    let mut memory = load_memory(&state.memory_dir, &payload.profile_id)
        .await
        .map_err(ApiError::internal)?;
    update_after_answer(
        &mut memory,
        question,
        &payload.selected_key,
        &correct_key,
        is_correct,
    );

    let context = build_ai_context(
        question,
        Some(payload.selected_key.clone()),
        Some(is_correct),
        summarize(&memory),
    );

    let (feedback, feedback_source, ai_error) = match state.ai_client.generate_hint(&context).await {
        Ok((hint, source)) => (Some(hint), Some(source), None),
        Err(error) => (None, None, Some(error.to_string())),
    };

    let submission = SubmittedAnswer {
        question_id: payload.question_id,
        selected_key: payload.selected_key.clone(),
        correct_key: correct_key.clone(),
        is_correct,
        feedback: feedback.clone(),
        ai_error: ai_error.clone(),
        submitted_at: Utc::now(),
    };

    let was_already_complete = record.completed_at.is_some();
    upsert_submission(&mut record, submission);

    if record.completed_at.is_some() && !was_already_complete {
        record_completed_session(
            &mut memory,
            RecentSession {
                exam_id: record.exam_id.clone(),
                score: record.submissions.iter().filter(|item| item.is_correct).count(),
                total_questions: record.question_ids.len(),
                completed_at: record.completed_at.unwrap_or_else(Utc::now),
            },
        );
    }

    save_memory(&state.memory_dir, &memory)
        .await
        .map_err(ApiError::internal)?;
    save_exam(&state, &record).await.map_err(ApiError::internal)?;

    let score = record.submissions.iter().filter(|item| item.is_correct).count();
    Ok(Json(SubmitAnswerResponse {
        exam_id: record.exam_id.clone(),
        question_id: payload.question_id,
        selected_key: payload.selected_key,
        correct_key,
        is_correct,
        correct_answer_text_original: question.answers[correct_index]
            .answer_text_original
            .clone(),
        feedback,
        feedback_source,
        ai_error,
        score,
        answered_count: record.submissions.len(),
        total_questions: record.question_ids.len(),
        dataset_correct_answers: dataset_correct_answers(&memory),
        total_dataset_questions: state.questions.len(),
        is_exam_complete: record.completed_at.is_some(),
    }))
}

async fn get_result(
    State(state): State<AppState>,
    Path(exam_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ExamResultResponse>, ApiError> {
    let record = load_exam(&state, &exam_id).await.map_err(ApiError::internal)?;
    let profile_id = params
        .get("profile_id")
        .map(|value| sanitize_profile_id(value))
        .unwrap_or_default();
    ensure_exam_access(&record, &profile_id)?;

    let score = record.submissions.iter().filter(|item| item.is_correct).count();
    let memory = load_memory(&state.memory_dir, &record.profile_id)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(ExamResultResponse {
        exam_id: record.exam_id.clone(),
        profile_id: record.profile_id.clone(),
        score,
        total_questions: record.question_ids.len(),
        passing_score: PASSING_SCORE,
        passed: score >= PASSING_SCORE,
        answered_count: record.submissions.len(),
        dataset_correct_answers: dataset_correct_answers(&memory),
        total_dataset_questions: state.questions.len(),
        incorrect_questions: review_items(&record, &state.questions, false),
    }))
}

async fn get_review(
    State(state): State<AppState>,
    Path(exam_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ReviewResponse>, ApiError> {
    let record = load_exam(&state, &exam_id).await.map_err(ApiError::internal)?;
    let profile_id = params
        .get("profile_id")
        .map(|value| sanitize_profile_id(value))
        .unwrap_or_default();
    ensure_exam_access(&record, &profile_id)?;

    let (correct_questions, incorrect_questions) = review_items_split(&record, &state.questions);
    Ok(Json(ReviewResponse {
        exam_id: record.exam_id.clone(),
        profile_id: record.profile_id.clone(),
        correct_questions,
        incorrect_questions,
    }))
}

async fn get_profile_review(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ProfileReviewResponse>, ApiError> {
    let profile_id = params
        .get("profile_id")
        .map(|value| sanitize_profile_id(value))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::bad_request("profile_id is required"))?;

    let records = load_profile_exams(&state, &profile_id)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(ProfileReviewResponse {
        profile_id,
        correct_questions: review_items_for_records(&records, &state.questions, true),
        incorrect_questions: review_items_for_records(&records, &state.questions, false),
    }))
}

async fn get_prompt_samples() -> Json<serde_json::Value> {
    let samples = prompt_samples();
    Json(serde_json::json!({
        "system_prompt": samples.system_prompt,
        "user_prompt": samples.user_prompt_template
    }))
}

async fn load_exam(state: &AppState, exam_id: &str) -> Result<ExamRecord> {
    let path = state.exams_dir.join(format!("{exam_id}.json"));
    read_json_file(&path).await
}

async fn save_exam(state: &AppState, record: &ExamRecord) -> Result<()> {
    let path = state.exams_dir.join(format!("{}.json", record.exam_id));
    write_json_file(&path, record).await
}

async fn load_profile_exams(state: &AppState, profile_id: &str) -> Result<Vec<ExamRecord>> {
    let mut exams = Vec::new();
    let mut entries = tokio::fs::read_dir(&*state.exams_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let record: ExamRecord = match read_json_file(&path).await {
            Ok(record) => record,
            Err(_) => continue,
        };
        if record.profile_id == profile_id {
            exams.push(record);
        }
    }

    Ok(exams)
}

fn question_by_id<'a>(questions: &'a [Question], question_id: u32) -> Result<&'a Question, ApiError> {
    questions
        .iter()
        .find(|question| question.id == question_id)
        .ok_or_else(|| ApiError::not_found("question not found"))
}

fn ensure_exam_access(record: &ExamRecord, profile_id: &str) -> Result<(), ApiError> {
    if record.profile_id == profile_id {
        Ok(())
    } else {
        Err(ApiError::forbidden("profile id does not match exam"))
    }
}

fn ensure_question_in_exam(record: &ExamRecord, question_id: u32) -> Result<(), ApiError> {
    if record.question_ids.contains(&question_id) {
        Ok(())
    } else {
        Err(ApiError::bad_request("question is not part of this exam"))
    }
}

fn sanitize_profile_id(input: &str) -> String {
    input
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .take(64)
        .collect()
}

fn generate_profile_id() -> String {
    format!(
        "profile-{}-{:08x}",
        Utc::now().timestamp_millis(),
        rand::rng().random::<u32>()
    )
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn internal(error: anyhow::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: error.to_string(),
        }
    }

    fn bad_request(message: &str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.to_string(),
        }
    }

    fn forbidden(message: &str) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: message.to_string(),
        }
    }

    fn not_found(message: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({
                "error": self.message
            })),
        )
            .into_response()
    }
}
