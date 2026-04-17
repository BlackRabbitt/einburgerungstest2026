use std::collections::{HashMap, HashSet};

use anyhow::{Result, anyhow};
use chrono::Utc;
use rand::prelude::SliceRandom;
use rand::{Rng, rng};

use crate::models::{
    AiAnswerContext, AiRequestContext, AnswerOptionPayload, ExamQuestionPayload, ExamRecord,
    MemorySummary, Question, ReviewItem, StartExamResponse, SubmittedAnswer,
};

pub const EXAM_QUESTION_COUNT: usize = 33;
pub const PASSING_SCORE: usize = 17;
pub const GENERAL_QUESTION_COUNT: usize = 30;
pub const BERLIN_QUESTION_COUNT: usize = 3;
pub const GENERAL_QUESTION_MAX_ID: u32 = 300;
pub const BERLIN_QUESTION_START_ID: u32 = 301;
pub const BERLIN_QUESTION_END_ID: u32 = 310;

pub fn validate_questions(questions: &[Question]) -> Result<()> {
    if questions.len() < EXAM_QUESTION_COUNT {
        return Err(anyhow!(
            "dataset must contain at least {} questions",
            EXAM_QUESTION_COUNT
        ));
    }

    let mut seen_ids = HashSet::new();
    for question in questions {
        if !seen_ids.insert(question.id) {
            return Err(anyhow!("duplicate question id {}", question.id));
        }
        if question.answers.len() != 4 {
            return Err(anyhow!("question {} must have exactly 4 answers", question.id));
        }
        let correct_count = question.answers.iter().filter(|answer| answer.correct).count();
        if correct_count != 1 {
            return Err(anyhow!(
                "question {} must have exactly 1 correct answer",
                question.id
            ));
        }
    }

    let general_count = questions.iter().filter(|question| is_general_question(question.id)).count();
    let berlin_count = questions.iter().filter(|question| is_berlin_question(question.id)).count();
    if general_count < GENERAL_QUESTION_COUNT {
        return Err(anyhow!(
            "dataset must contain at least {} general questions",
            GENERAL_QUESTION_COUNT
        ));
    }
    if berlin_count < BERLIN_QUESTION_COUNT {
        return Err(anyhow!(
            "dataset must contain at least {} Berlin-specific questions",
            BERLIN_QUESTION_COUNT
        ));
    }

    Ok(())
}

const PRIORITY_GENERAL_LIMIT: usize = 12;
const PRIORITY_BERLIN_LIMIT: usize = BERLIN_QUESTION_COUNT;

pub fn create_exam(
    profile_id: String,
    questions: &[Question],
    prioritized_question_ids: &[u32],
    excluded_question_ids: &HashSet<u32>,
    dataset_correct_answers: usize,
) -> (ExamRecord, StartExamResponse) {
    let mut rng = rng();

    let mut general_questions = select_questions_from_pool(
        questions,
        prioritized_question_ids,
        excluded_question_ids,
        GENERAL_QUESTION_COUNT,
        PRIORITY_GENERAL_LIMIT,
        is_general_question,
        &mut rng,
    );
    let mut berlin_questions = select_questions_from_pool(
        questions,
        prioritized_question_ids,
        excluded_question_ids,
        BERLIN_QUESTION_COUNT,
        PRIORITY_BERLIN_LIMIT,
        is_berlin_question,
        &mut rng,
    );

    general_questions.append(&mut berlin_questions);
    general_questions.shuffle(&mut rng);

    let exam_id = format!("exam-{}-{:08x}", Utc::now().timestamp_millis(), rng.random::<u32>());

    let record = ExamRecord {
        exam_id: exam_id.clone(),
        profile_id: profile_id.clone(),
        question_ids: general_questions.iter().map(|question| question.id).collect(),
        submissions: Vec::new(),
        started_at: Utc::now(),
        completed_at: None,
    };

    let response = StartExamResponse {
        exam_id,
        profile_id,
        total_questions: EXAM_QUESTION_COUNT,
        passing_score: PASSING_SCORE,
        dataset_correct_answers,
        total_dataset_questions: questions.len(),
        questions: general_questions.into_iter().map(to_exam_payload).collect(),
    };

    (record, response)
}

fn select_questions_from_pool<F>(
    questions: &[Question],
    prioritized_question_ids: &[u32],
    excluded_question_ids: &HashSet<u32>,
    target_count: usize,
    priority_limit: usize,
    pool_filter: F,
    rng: &mut impl rand::Rng,
) -> Vec<Question>
where
    F: Fn(u32) -> bool,
{
    let priority_set: HashSet<u32> = prioritized_question_ids.iter().copied().collect();

    let mut prioritized_pool = questions
        .iter()
        .filter(|question| {
            pool_filter(question.id)
                && priority_set.contains(&question.id)
                && !excluded_question_ids.contains(&question.id)
        })
        .cloned()
        .collect::<Vec<_>>();
    prioritized_pool.shuffle(rng);

    let mut selected = prioritized_pool
        .into_iter()
        .take(priority_limit.min(target_count))
        .collect::<Vec<_>>();
    let selected_ids = selected.iter().map(|question| question.id).collect::<HashSet<_>>();

    let mut remaining = questions
        .iter()
        .filter(|question| {
            pool_filter(question.id)
                && !selected_ids.contains(&question.id)
                && !excluded_question_ids.contains(&question.id)
        })
        .cloned()
        .collect::<Vec<_>>();
    remaining.shuffle(rng);
    selected.extend(
        remaining
            .into_iter()
            .take(target_count.saturating_sub(selected.len())),
    );

    selected
}

pub fn is_general_question(question_id: u32) -> bool {
    question_id <= GENERAL_QUESTION_MAX_ID
}

pub fn is_berlin_question(question_id: u32) -> bool {
    (BERLIN_QUESTION_START_ID..=BERLIN_QUESTION_END_ID).contains(&question_id)
}

pub fn to_exam_payload(question: Question) -> ExamQuestionPayload {
    ExamQuestionPayload {
        id: question.id,
        text_original: question.text_original,
        text_translation: question.text_translation,
        answers: question
            .answers
            .into_iter()
            .enumerate()
            .map(|(index, answer)| AnswerOptionPayload {
                key: answer_key(index),
                text_original: answer.answer_text_original,
                text_translation: answer.answer_text_translation,
            })
            .collect(),
    }
}

pub fn answer_key(index: usize) -> String {
    match index {
        0 => "A",
        1 => "B",
        2 => "C",
        3 => "D",
        _ => "?",
    }
    .to_string()
}

pub fn answer_index(key: &str) -> Option<usize> {
    match key.trim().to_uppercase().as_str() {
        "A" => Some(0),
        "B" => Some(1),
        "C" => Some(2),
        "D" => Some(3),
        _ => None,
    }
}

pub fn correct_answer(question: &Question) -> (usize, String) {
    let index = question
        .answers
        .iter()
        .position(|answer| answer.correct)
        .expect("validated dataset must have a correct answer");
    (index, answer_key(index))
}

pub fn build_ai_context(
    question: &Question,
    selected_key: Option<String>,
    is_correct: Option<bool>,
    memory_summary: MemorySummary,
) -> AiRequestContext {
    let (_, correct_key) = correct_answer(question);

    AiRequestContext {
        question_id: question.id,
        question_text_original: question.text_original.clone(),
        question_text_translation: question.text_translation.clone(),
        answers: question
            .answers
            .iter()
            .enumerate()
            .map(|(index, answer)| AiAnswerContext {
                key: answer_key(index),
                answer_text_original: answer.answer_text_original.clone(),
                answer_text_translation: if answer.answer_text_translation.trim().is_empty() {
                    answer.answer_text_original.clone()
                } else {
                    answer.answer_text_translation.clone()
                },
            })
            .collect(),
        correct_key,
        selected_key,
        is_correct,
        memory_summary,
    }
}

pub fn upsert_submission(record: &mut ExamRecord, submission: SubmittedAnswer) {
    if let Some(existing) = record
        .submissions
        .iter_mut()
        .find(|item| item.question_id == submission.question_id)
    {
        *existing = submission;
    } else {
        record.submissions.push(submission);
    }

    if record.submissions.len() == record.question_ids.len() {
        record.completed_at = Some(Utc::now());
    }
}

pub fn review_items(record: &ExamRecord, questions: &[Question], is_correct: bool) -> Vec<ReviewItem> {
    review_items_for_records(std::slice::from_ref(record), questions, is_correct)
}

pub fn review_items_for_records(records: &[ExamRecord], questions: &[Question], is_correct: bool) -> Vec<ReviewItem> {
    let question_map: HashMap<u32, &Question> = questions.iter().map(|q| (q.id, q)).collect();

    // Keep only the latest submission per question to remove duplicates across exams.
    let mut latest: HashMap<u32, &SubmittedAnswer> = HashMap::new();
    for record in records {
        for submission in record.submissions.iter().filter(|s| s.is_correct == is_correct) {
            latest
                .entry(submission.question_id)
                .and_modify(|prev| {
                    if submission.submitted_at > prev.submitted_at {
                        *prev = submission;
                    }
                })
                .or_insert(submission);
        }
    }

    let mut items: Vec<ReviewItem> = latest
        .into_values()
        .filter_map(|submission| question_map.get(&submission.question_id).map(|&q| build_review_item(q, submission)))
        .collect();

    items.sort_by(|a, b| b.submitted_at.cmp(&a.submitted_at));
    items
}

pub fn review_items_split(record: &ExamRecord, questions: &[Question]) -> (Vec<ReviewItem>, Vec<ReviewItem>) {
    let question_map: HashMap<u32, &Question> = questions.iter().map(|q| (q.id, q)).collect();
    let mut correct = Vec::new();
    let mut incorrect = Vec::new();
    for submission in &record.submissions {
        if let Some(&question) = question_map.get(&submission.question_id) {
            let item = build_review_item(question, submission);
            if submission.is_correct {
                correct.push(item);
            } else {
                incorrect.push(item);
            }
        }
    }
    correct.sort_by_key(|item| std::cmp::Reverse(item.question_id));
    incorrect.sort_by_key(|item| std::cmp::Reverse(item.question_id));
    (correct, incorrect)
}

fn build_review_item(question: &Question, submission: &SubmittedAnswer) -> ReviewItem {
    let selected_index = answer_index(&submission.selected_key).unwrap_or(0);
    let correct_index = answer_index(&submission.correct_key).unwrap_or(0);
    ReviewItem {
        question_id: question.id,
        text_original: question.text_original.clone(),
        text_translation: question.text_translation.clone(),
        selected_key: submission.selected_key.clone(),
        selected_answer_text_original: question.answers[selected_index].answer_text_original.clone(),
        correct_key: submission.correct_key.clone(),
        correct_answer_text_original: question.answers[correct_index].answer_text_original.clone(),
        is_correct: submission.is_correct,
        feedback_text: submission
            .feedback
            .as_ref()
            .map(|feedback| feedback.feedback_text.clone())
            .unwrap_or_default(),
        memory_trick: submission
            .feedback
            .as_ref()
            .map(|feedback| feedback.memory_trick.clone())
            .unwrap_or_default(),
        ai_error: submission.ai_error.clone(),
        submitted_at: submission.submitted_at,
        answers: question
            .answers
            .iter()
            .enumerate()
            .map(|(i, a)| AnswerOptionPayload {
                key: answer_key(i),
                text_original: a.answer_text_original.clone(),
                text_translation: a.answer_text_translation.clone(),
            })
            .collect(),
    }
}
