# German Einbuergerungstest Mock Exam

Simple Rust + Axum web app for practicing the German naturalization test in a mock-exam flow. The exam stays in German. Hints, explanations, and corrections use easy English plus a few important German words.

## Features

- 33-question mock exam from the local JSON dataset, with future exams biased toward your previously missed questions
- Exactly 1 question at a time, with dataset-driven grading only
- Optional per-question emoji mode and hint sidebar
- AI tutor via OpenAI for hints, feedback, and memory tricks
- Start screen asks for a profile ID; using the same ID on laptop and mobile shares server-side learning history
- Browser `localStorage` persistence for the current exam
- Server-side JSON memory files for lightweight learner history
- Final score, pass/fail, and review of incorrect answers

## Install and run

1. Make sure Rust is installed.
2. Optionally create a `.env` file from `.env.example`.
3. From the project root, run:

```bash
cargo run
```

4. Open `http://127.0.0.1:3000`.
5. Enter a `Profile ID` on the start screen. Use the same ID on laptop and mobile if you want shared learning history across devices.

To access it from your phone on the same Wi-Fi, set `HOST=0.0.0.0` and open `http://<your-laptop-local-ip>:3000`.
Or, get your laptop hostname and from your phone, try: `http://<your-mac-name>.local:3000`

How to find your hostname on the Mac:
```
scutil --get LocalHostName # get your laptop hostname
```



The app looks for the question dataset in this order:

1. `data/questions.json`
2. `questions-answers-dataset.json`

This repo already includes `questions-answers-dataset.json`, so the app runs without any file moves.

Dataset attribution: the included question dataset was copied from https://gor-sg.github.io/btest/questions.js

## OpenAI setup

The quiz works without OpenAI. If you want AI hints:

Create a `.env` file in the project root:

```env
OPENAI_API_KEY=your_key_here
OPENAI_MODEL=gpt-5.4
PORT=3000
```

Or export variables directly:

```bash
export OPENAI_API_KEY=your_key_here
export OPENAI_MODEL=gpt-5.4
cargo run
```

Supported `.env` variables:

- `OPENAI_API_KEY`
- `OPENAI_MODEL`
- `HOST`
- `PORT`
- `RUST_LOG`

`OPENAI_MODEL` is optional. If unset, the backend defaults to `gpt-5.4`.

## Dataset schema

The backend uses the dataset schema directly. Each question must look like this:

```json
{
  "id": 286,
  "text_original": "Welche Organisation ... ?",
  "text_translation": "Which organization ... ?",
  "answers": [
    {
      "answer_text_original": "der Betriebsrat",
      "answer_text_translation": "the works council",
      "correct": true
    }
  ]
}
```

Requirements enforced by the backend:

- Every question must have exactly 4 answers.
- Every question must have exactly 1 correct answer.
- Correctness is always determined from the dataset only.
- AI never decides whether an answer is correct.

## Replace or add `questions.json`

You can either:

- replace `data/questions.json`, or
- replace `questions-answers-dataset.json`

As long as the JSON matches the schema above, no adapter is needed.

## How correctness works

- The UI shows only `text_original` and `answer_text_original` in exam mode.
- On submit, the backend compares the selected answer key against the dataset's `correct: true` entry.
- The correct answer text returned to the client also comes from the dataset.
- OpenAI only receives the correctness result as input for explanation purposes.

## Browser localStorage

The frontend stores:

- `profileId`
- selected randomized 33-question exam payload
- current question index
- submitted answers
- per-question hint cache
- per-question UI toggle state

Refreshing the page restores the current exam screen from browser storage. This storage is browser-specific, so laptop and mobile do not share in-progress exam state automatically.

## Server-side memory

Server-side memory is stored in JSON files under `data/memory/`.

If you want to backfill dataset-progress coverage from old exam records for existing profiles, run one of these commands manually:

```bash
cargo run --bin backfill_progress
```

```bash
cargo run --bin backfill_progress PROFILE_ID
```

This is a manual one-time maintenance command. It does not run automatically when a profile loads.

Each profile file tracks:

- frequently wrong question ids
- repeated confusion patterns like `B -> A`
- vocabulary weakness tokens
- recent completed sessions

This memory is only extra context for hints and exam selection bias. It never overrides grading. If two devices use the same `profile_id`, they share this server-side memory and session history. Future mock exams for that profile prioritize a limited set of frequently missed questions, then fill the rest randomly.

Exam session records are stored under `data/exams/`.

## API overview

- `POST /api/profile/init`
- `GET /api/exam/start?profile_id=...`
- `POST /api/exam/hint`
- `POST /api/exam/answer`
- `GET /api/exam/result/:id?profile_id=...`
- `GET /api/exam/review/:id?profile_id=...`
- `GET /api/meta/prompts`

## Architecture

- `src/main.rs`: bootstraps Axum, static files, and shared state
- `src/routes.rs`: HTTP endpoints and request flow
- `src/models.rs`: dataset, API, exam, memory, and AI payload structs
- `src/quiz.rs`: random exam creation, grading helpers, and review shaping
- `src/ai.rs`: OpenAI integration for hints, feedback, emoji cues, and missing translations
- `src/memory.rs`: learner memory loading, updating, and summarizing
- `src/storage.rs`: JSON file helpers
- `static/`: dependency-free HTML, CSS, and JavaScript frontend

## Sample OpenAI prompts

Example system prompt:

```text
You are a concise tutor for beginners preparing for the German Einbuergerungstest.
You are not the judge of correctness.
The quiz stays in German.
Explanations are short, beginner-friendly, and mostly English with important German words preserved.
Return valid JSON only.
```

Example user prompt shape:

```text
Question id: 286
German question: Welche Organisation in einer Firma hilft ...
English question: Which organization in a company helps employees ...
Answers: A: der Betriebsrat (the works council); B: ...
Correct answer key: A
Selected answer key: B
Result status: incorrect
Memory summary: frequent wrong question ids = [286]; concept confusions = ["B -> A"]; vocabulary weaknesses = ["firma", "probleme"]
```

You can also inspect the live prompt sample from:

```bash
curl http://127.0.0.1:3000/api/meta/prompts
```

## Assumptions made

- A lightweight profile-ID flow is enough; no login system was added.
- Cross-device sharing applies to server-side memory/history, not live browser `localStorage` exam state.
- The provided dataset of 310 questions is the full local pool.
- Hint generation before answering is supported through a small extra endpoint, not by changing the core grading flow.
- The app binds to `127.0.0.1:3000` by default.

