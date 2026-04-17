#![allow(dead_code)]
use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::Result;
use tokio::fs;

#[path = "../memory.rs"]
mod memory;
#[path = "../models.rs"]
mod models;
#[path = "../storage.rs"]
mod storage;

use memory::backfill_correct_progress_for_profile;
use models::ExamRecord;
use storage::{read_json_file, root_data_dir};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let data_root = root_data_dir();
    let memory_dir = data_root.join("memory");
    let exams_dir = data_root.join("exams");

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("--help") {
        print_help();
        return Ok(());
    }

    let profiles = if let Some(profile_id) = args.first() {
        vec![profile_id.clone()]
    } else {
        discover_profiles(&exams_dir).await?
    };

    if profiles.is_empty() {
        println!("No profiles found to backfill.");
        return Ok(());
    }

    for profile_id in profiles {
        let memory = backfill_correct_progress_for_profile(&memory_dir, &exams_dir, &profile_id).await?;
        println!(
            "Backfilled {}: {} correctly answered questions",
            profile_id,
            memory.correctly_answered_question_ids.len()
        );
    }

    Ok(())
}

async fn discover_profiles(exams_dir: &PathBuf) -> Result<Vec<String>> {
    let mut profiles = BTreeSet::new();
    let mut entries = match fs::read_dir(exams_dir).await {
        Ok(entries) => entries,
        Err(_) => return Ok(Vec::new()),
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
        profiles.insert(record.profile_id);
    }

    Ok(profiles.into_iter().collect())
}

fn print_help() {
    println!("Usage:");
    println!("  cargo run --bin backfill_progress            # backfill all profiles from exam records");
    println!("  cargo run --bin backfill_progress PROFILE_ID # backfill one profile only");
}
