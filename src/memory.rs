use std::cmp::Reverse;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use tokio::fs;

use crate::models::{ExamRecord, LearnerMemory, MemorySummary, Question, RecentSession};
use crate::storage::{file_exists, read_json_file, write_json_file};

const RECOVERY_CORRECT_STREAK: u32 = 3;
const WRONG_QUESTION_COOLDOWN_EXAMS: u32 = 2;

pub fn memory_path(memory_dir: &Path, profile_id: &str) -> PathBuf {
    memory_dir.join(format!("{profile_id}.json"))
}

pub async fn load_memory(memory_dir: &Path, profile_id: &str) -> Result<LearnerMemory> {
    let path = memory_path(memory_dir, profile_id);
    if file_exists(&path).await {
        read_json_file(&path).await
    } else {
        Ok(LearnerMemory::empty(profile_id.to_string()))
    }
}

pub async fn save_memory(memory_dir: &Path, memory: &LearnerMemory) -> Result<()> {
    let path = memory_path(memory_dir, &memory.profile_id);
    write_json_file(&path, memory).await
}

pub fn summarize(memory: &LearnerMemory) -> MemorySummary {
    MemorySummary {
        frequent_wrong_questions: top_question_ids(&memory.frequently_wrong_question_ids, 5),
        concept_confusions: top_keys(&memory.concept_confusions, 5),
        vocabulary_weaknesses: top_keys(&memory.vocabulary_weaknesses, 6),
    }
}

pub fn prioritized_question_ids(memory: &LearnerMemory, limit: usize) -> Vec<u32> {
    top_question_ids(&memory.frequently_wrong_question_ids, limit)
        .into_iter()
        .filter(|question_id| !is_on_cooldown(memory, *question_id))
        .collect()
}

pub fn dataset_correct_answers(memory: &LearnerMemory) -> usize {
    memory.correctly_answered_question_ids.len()
}

#[allow(dead_code)]
pub async fn backfill_correct_progress_for_profile(
    memory_dir: &Path,
    exams_dir: &Path,
    profile_id: &str,
) -> Result<LearnerMemory> {
    let mut memory = load_memory(memory_dir, profile_id).await?;
    let mut restored = memory
        .correctly_answered_question_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>();

    let mut entries = match fs::read_dir(exams_dir).await {
        Ok(entries) => entries,
        Err(_) => {
            memory.correctly_answered_question_ids = restored.into_iter().collect();
            memory.correctly_answered_question_ids.sort_unstable();
            save_memory(memory_dir, &memory).await?;
            return Ok(memory);
        }
    };

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let record = match read_json_file::<ExamRecord>(&path).await {
            Ok(record) => record,
            Err(_) => continue,
        };
        if record.profile_id != memory.profile_id {
            continue;
        }

        for submission in record.submissions.iter().filter(|item| item.is_correct) {
            restored.insert(submission.question_id);
        }
    }

    memory.correctly_answered_question_ids = restored.into_iter().collect();
    memory.correctly_answered_question_ids.sort_unstable();
    save_memory(memory_dir, &memory).await?;
    Ok(memory)
}

pub fn update_after_answer(
    memory: &mut LearnerMemory,
    question: &Question,
    selected_key: &str,
    correct_key: &str,
    is_correct: bool,
) {
    if is_correct {
        if let Err(pos) = memory.correctly_answered_question_ids.binary_search(&question.id) {
            memory.correctly_answered_question_ids.insert(pos, question.id);
        }
        if memory.frequently_wrong_question_ids.contains_key(&question.id) {
            let streak = memory.recovery_streaks.entry(question.id).or_insert(0);
            *streak += 1;

            if *streak >= RECOVERY_CORRECT_STREAK {
                decrement_question_weight(&mut memory.frequently_wrong_question_ids, question.id);
                for token in important_tokens(question).into_iter().take(4) {
                    decrement_string_weight(&mut memory.vocabulary_weaknesses, &token);
                }
                memory.recovery_streaks.remove(&question.id);
            }
        } else {
            memory.recovery_streaks.remove(&question.id);
        }
        return;
    }

    *memory
        .frequently_wrong_question_ids
        .entry(question.id)
        .or_insert(0) += 1;
    memory.recovery_streaks.remove(&question.id);
    memory
        .question_cooldowns
        .insert(question.id, WRONG_QUESTION_COOLDOWN_EXAMS);

    let confusion_key = format!("{selected_key} -> {correct_key}");
    *memory.concept_confusions.entry(confusion_key).or_insert(0) += 1;

    for token in important_tokens(question).into_iter().take(4) {
        *memory.vocabulary_weaknesses.entry(token).or_insert(0) += 1;
    }
}

fn decrement_question_weight(map: &mut std::collections::HashMap<u32, u32>, key: u32) {
    if let Some(count) = map.get_mut(&key) {
        if *count > 1 {
            *count -= 1;
        } else {
            map.remove(&key);
        }
    }
}

fn decrement_string_weight(map: &mut std::collections::HashMap<String, u32>, key: &str) {
    if let Some(count) = map.get_mut(key) {
        if *count > 1 {
            *count -= 1;
        } else {
            map.remove(key);
        }
    }
}

pub fn record_completed_session(memory: &mut LearnerMemory, session: RecentSession) {
    memory.recent_sessions.push(session);
    memory
        .recent_sessions
        .sort_by_key(|item| Reverse(item.completed_at));
    memory.recent_sessions.truncate(10);
}

pub fn advance_question_cooldowns(memory: &mut LearnerMemory) {
    memory.question_cooldowns.retain(|_, remaining| {
        if *remaining > 1 {
            *remaining -= 1;
            true
        } else {
            false
        }
    });
}

pub fn cooldown_question_ids(memory: &LearnerMemory) -> HashSet<u32> {
    memory
        .question_cooldowns
        .iter()
        .filter(|(_, remaining)| **remaining > 0)
        .map(|(question_id, _)| *question_id)
        .collect()
}

fn important_tokens(question: &Question) -> Vec<String> {
    let text = format!(
        "{} {}",
        question.text_original,
        question
            .answers
            .iter()
            .find(|answer| answer.correct)
            .map(|answer| answer.answer_text_original.as_str())
            .unwrap_or("")
    );

    let mut tokens = text
        .split(|character: char| !character.is_alphabetic() && character != '-')
        .map(|token| token.trim().to_lowercase())
        .filter(|token| token.len() >= 5)
        .filter(|token| {
            !matches!(
                token.as_str(),
                "welche"
                    | "deutschland"
                    | "menschen"
                    | "arbeitnehmern"
                    | "arbeitnehmerinnen"
                    | "arbeitgeber"
                    | "arbeitgeberin"
            )
        })
        .collect::<Vec<_>>();

    tokens.sort();
    tokens.dedup();
    tokens
}

fn top_question_ids(map: &std::collections::HashMap<u32, u32>, limit: usize) -> Vec<u32> {
    let mut entries = map.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(key, count)| (Reverse(**count), **key));
    entries.into_iter().take(limit).map(|(key, _)| *key).collect()
}

fn top_keys(map: &std::collections::HashMap<String, u32>, limit: usize) -> Vec<String> {
    let mut entries = map.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(key, count)| (Reverse(**count), (*key).clone()));
    entries
        .into_iter()
        .take(limit)
        .map(|(key, _)| key.clone())
        .collect()
}

fn is_on_cooldown(memory: &LearnerMemory, question_id: u32) -> bool {
    memory
        .question_cooldowns
        .get(&question_id)
        .copied()
        .unwrap_or(0)
        > 0
}
