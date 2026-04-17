const STORAGE_KEY = "naturalization-mock-exam-state-v2";

const state = {
  profileId: null,
  aiStatus: null,
  exam: null,
  datasetProgress: { correct: 0, total: 0 },
  historyFilter: "incorrect",
  historyReview: { correct: [], incorrect: [] },
  currentIndex: 0,
  submissions: {},
  hintCache: {},
  hintSourceByQuestion: {},
  aiErrorByQuestion: {},
  generatedTranslationsByQuestion: {},
  uiByQuestion: {},
  activeQuestionHint: null,
  hintLoadingQuestionId: null,
  practice: { questions: [], index: 0, answers: {} },
};

const el = {
  startScreen: document.getElementById("start-screen"),
  examScreen: document.getElementById("exam-screen"),
  resultScreen: document.getElementById("result-screen"),
  historyScreen: document.getElementById("history-screen"),
  startExamBtn: document.getElementById("start-exam-btn"),
  profileStrip: document.getElementById("profile-strip"),
  profileDisplay: document.getElementById("profile-display"),
  changeProfileBtn: document.getElementById("change-profile-btn"),
  viewHistoryBtn: document.getElementById("view-history-btn"),
  profileIdInput: document.getElementById("profile-id-input"),
  restartBtn: document.getElementById("restart-btn"),
  restartExamBtn: document.getElementById("restart-exam-btn"),
  startAiBadge: document.getElementById("start-ai-badge"),
  examLayout: document.getElementById("exam-layout"),
  progressLabel: document.getElementById("progress-label"),
  datasetProgressText: document.getElementById("dataset-progress-text"),
  datasetProgressFill: document.getElementById("dataset-progress-fill"),
  questionText: document.getElementById("question-text"),
  translationToggle: document.getElementById("translation-toggle"),
  answerForm: document.getElementById("answer-form"),
  backBtn: document.getElementById("back-btn"),
  submitBtn: document.getElementById("submit-btn"),
  nextBtn: document.getElementById("next-btn"),
  emojiToggle: document.getElementById("emoji-toggle"),
  hintToggle: document.getElementById("hint-toggle"),
  hintBackdrop: document.getElementById("hint-backdrop"),
  hintSidebar: document.getElementById("hint-sidebar"),
  hintCloseBtn: document.getElementById("hint-close-btn"),
  hintTitle: document.getElementById("hint-title"),
  hintText: document.getElementById("hint-text"),
  memoryTrick: document.getElementById("memory-trick"),
  feedbackBox: document.getElementById("feedback-box"),
  aiBadge: document.getElementById("ai-badge"),
  aiBadgeText: document.getElementById("ai-badge-text"),
  aiBadgeSpinner: document.getElementById("ai-badge-spinner"),
  resultHeadline: document.getElementById("result-headline"),
  resultCopy: document.getElementById("result-copy"),
  resultSummary: document.getElementById("result-summary"),
  reviewList: document.getElementById("review-list"),
  historyList: document.getElementById("history-list"),
  historyCorrectBtn: document.getElementById("history-correct-btn"),
  historyIncorrectBtn: document.getElementById("history-incorrect-btn"),
  historyBackBtn: document.getElementById("history-back-btn"),
  retryMistakesBtn: document.getElementById("retry-mistakes-btn"),
  historyBackBtnBottom: document.getElementById("history-back-btn-bottom"),
  retryMistakesBtnBottom: document.getElementById("retry-mistakes-btn-bottom"),
  practiceScreen: document.getElementById("practice-screen"),
  practiceQuestionArea: document.getElementById("practice-question-area"),
  practiceCompleteArea: document.getElementById("practice-complete-area"),
  practiceProgressLabel: document.getElementById("practice-progress-label"),
  practiceQuestionText: document.getElementById("practice-question-text"),
  practiceAnswerForm: document.getElementById("practice-answer-form"),
  practiceSubmitBtn: document.getElementById("practice-submit-btn"),
  practiceNextBtn: document.getElementById("practice-next-btn"),
  practiceFeedbackBox: document.getElementById("practice-feedback-box"),
  practiceScoreHeadline: document.getElementById("practice-score-headline"),
  practiceScoreCopy: document.getElementById("practice-score-copy"),
  practiceAgainBtn: document.getElementById("practice-again-btn"),
  practiceBackBtn: document.getElementById("practice-back-btn"),
};

document.addEventListener("DOMContentLoaded", init);

async function init() {
  bindEvents();
  restoreState();
  await loadAiStatus();

  if (el.profileIdInput) {
    el.profileIdInput.value = state.profileId || "";
  }
  renderProfileStrip();

  if (state.exam && state.profileId) {
    if (isExamFinishedLocally()) {
      await renderResultScreen();
    } else {
      renderExamScreen();
    }
  } else {
    showPanel("start");
  }
}

function bindEvents() {
  el.startExamBtn.addEventListener("click", startExam);
  el.restartBtn.addEventListener("click", startExam);
  el.changeProfileBtn.addEventListener("click", changeProfile);
  el.viewHistoryBtn.addEventListener("click", openHistoryScreen);
  el.restartExamBtn.addEventListener("click", startExam);
  el.backBtn.addEventListener("click", goToPreviousQuestion);
  el.submitBtn.addEventListener("click", submitCurrentAnswer);
  el.nextBtn.addEventListener("click", goToNextQuestion);
  el.translationToggle.addEventListener("change", async () => {
    persistUiState();
    if (el.translationToggle.checked) {
      await ensureGeneratedTranslationsForCurrentQuestion();
    }
    renderQuestion();
  });
  el.emojiToggle.addEventListener("change", async () => {
    persistUiState();
    if (el.emojiToggle.checked) {
      await ensureHintForCurrentQuestion();
    }
    renderQuestion();
  });
  el.hintToggle.addEventListener("change", async () => {
    persistUiState();
    if (el.hintToggle.checked) {
      await ensureHintForCurrentQuestion();
    }
    renderHintSidebar();
    saveState();
  });
  el.hintCloseBtn.addEventListener("click", closeHintSidebar);
  el.hintBackdrop.addEventListener("click", closeHintSidebar);
  el.historyCorrectBtn.addEventListener("click", () => setHistoryFilter("correct"));
  el.historyIncorrectBtn.addEventListener("click", () => setHistoryFilter("incorrect"));
  el.historyBackBtn.addEventListener("click", goBackFromHistory);
  el.retryMistakesBtn.addEventListener("click", startPractice);
  el.historyBackBtnBottom.addEventListener("click", goBackFromHistory);
  el.retryMistakesBtnBottom.addEventListener("click", startPractice);
  el.practiceSubmitBtn.addEventListener("click", submitPracticeAnswer);
  el.practiceNextBtn.addEventListener("click", nextPracticeQuestion);
  el.practiceAgainBtn.addEventListener("click", startPractice);
  el.practiceBackBtn.addEventListener("click", () => showPanel("history"));
}

async function ensureProfile(profileId) {
  const response = await fetchJson("/api/profile/init", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ profile_id: profileId }),
  });
  state.profileId = response.profile_id;
  if (el.profileIdInput) {
    el.profileIdInput.value = state.profileId;
  }
  renderProfileStrip();
  renderDatasetProgress();
  saveState();
}

function renderProfileStrip() {
  const hasProfile = Boolean(state.profileId);
  el.profileStrip.classList.toggle("hidden", !hasProfile);
  el.profileDisplay.textContent = hasProfile ? `Profile: ${state.profileId}` : "Profile: not set";
  renderDatasetProgress();
}

function resetLocalExamState() {
  state.exam = null;
  state.currentIndex = 0;
  state.historyFilter = "incorrect";
  state.historyReview = { correct: [], incorrect: [] };
  state.datasetProgress = { correct: 0, total: 0 };
  state.submissions = {};
  state.hintCache = {};
  state.hintSourceByQuestion = {};
  state.aiErrorByQuestion = {};
  state.generatedTranslationsByQuestion = {};
  state.uiByQuestion = {};
  state.activeQuestionHint = null;
  state.hintLoadingQuestionId = null;
}

async function changeProfile() {
  const currentValue = state.profileId || "";
  const nextValue = window.prompt("Enter a profile ID. Use the same ID on phone and laptop to share learning history.", currentValue);
  if (nextValue === null) return;

  const trimmed = nextValue.trim();
  if (!trimmed) {
    window.alert("Profile ID cannot be empty.");
    return;
  }

  if (trimmed === currentValue) return;

  if (state.exam && !window.confirm("Changing profile will reset this device's current exam and switch to the new profile. Continue?")) {
    return;
  }

  await ensureProfile(trimmed);
  resetLocalExamState();
  saveState();
  showPanel("start");
}

async function loadAiStatus() {
  state.aiStatus = await fetchJson("/api/ai/status");
  updateAiBadges();
  saveState();
}

async function startExam() {
  const rawProfileId = el.profileIdInput ? el.profileIdInput.value.trim() : state.profileId;
  if (!rawProfileId) {
    window.alert("Please enter a profile ID. Use the same one on both devices to share progress.");
    el.profileIdInput?.focus();
    return;
  }

  await ensureProfile(rawProfileId);
  const data = await fetchJson(`/api/exam/start?profile_id=${encodeURIComponent(state.profileId)}`);
  state.exam = data;
  state.currentIndex = 0;
  state.historyFilter = "incorrect";
  state.historyReview = { correct: [], incorrect: [] };
  state.datasetProgress = {
    correct: data.dataset_correct_answers ?? 0,
    total: data.total_dataset_questions ?? 0,
  };
  renderDatasetProgress();
  state.submissions = {};
  state.hintCache = {};
  state.hintSourceByQuestion = {};
  state.aiErrorByQuestion = {};
  state.generatedTranslationsByQuestion = {};
  state.uiByQuestion = {};
  state.activeQuestionHint = null;
  saveState();
  renderExamScreen();
}

function renderExamScreen() {
  if (!state.exam) return;
  showPanel("exam");
  renderQuestion();
}

function renderQuestion() {
  const question = currentQuestion();
  if (!question) return;

  renderDatasetProgress();

  const ui = uiForQuestion(question.id);
  const submission = state.submissions[question.id];
  const hint = state.hintCache[question.id] || submission?.feedback || null;
  const hintSource = state.hintSourceByQuestion[question.id] || submission?.feedback_source || null;
  const aiError = state.aiErrorByQuestion[question.id] || submission?.ai_error || null;
  const isHintLoadingForQuestion = state.hintLoadingQuestionId === question.id;
  state.activeQuestionHint = hint;

  el.progressLabel.textContent = `Frage ${state.currentIndex + 1} / ${state.exam.total_questions}`;
  el.questionText.textContent =
    canUseAiFeatures() && ui.emojiEnabled && hint?.emojiQuestion ? hint.emojiQuestion : question.text_original;
  renderQuestionTranslation(question, ui.translationOpen);

  el.translationToggle.checked = Boolean(ui.translationOpen);
  el.emojiToggle.checked = canUseAiFeatures() && ui.emojiEnabled;
  el.hintToggle.checked = canUseAiFeatures() && ui.hintOpen;
  el.emojiToggle.disabled = !canUseAiFeatures() || isHintLoadingForQuestion;
  el.hintToggle.disabled = !canUseAiFeatures() || isHintLoadingForQuestion;
  el.emojiToggle.closest(".switch").classList.toggle("disabled", !canUseAiFeatures());
  el.hintToggle.closest(".switch").classList.toggle("disabled", !canUseAiFeatures());
  updateAiBadges(hintSource, aiError, isHintLoadingForQuestion);

  renderAnswerOptions(question, submission, hint);
  renderHintSidebar(aiError);
  renderFeedback(submission, aiError);

  el.backBtn.disabled = state.currentIndex === 0 || isSubmitting();
  el.submitBtn.classList.toggle("hidden", Boolean(submission));
  el.submitBtn.disabled = isSubmitting();
  el.nextBtn.classList.toggle("hidden", !submission);
  el.nextBtn.disabled = isSubmitting();
}

function renderAnswerOptions(question, submission, hint) {
  const selectedKey = submission?.selected_key || null;
  const showTranslation = Boolean(uiForQuestion(question.id).translationOpen);
  const generatedTranslations = state.generatedTranslationsByQuestion[question.id] || {};
  const renderedAnswers = question.answers.map((answer) => {
    const emojiOption =
      canUseAiFeatures() && uiForQuestion(question.id).emojiEnabled && hint?.emojiOptions?.length === 4
        ? hint.emojiOptions.find((option) => option.key === answer.key)
        : null;

    return {
      key: answer.key,
      text: emojiOption?.text || answer.text_original,
      translation: answer.text_translation || generatedTranslations[answer.key] || "",
      translationSource: answer.text_translation ? "dataset" : generatedTranslations[answer.key] ? "ai" : null,
    };
  });

  el.answerForm.innerHTML = "";
  renderedAnswers.forEach((answer) => {
    const label = document.createElement("label");
    const classes = ["answer-option"];
    if (selectedKey === answer.key) classes.push("selected");
    if (submission) {
      if (submission.correct_key === answer.key) classes.push("correct");
      else if (submission.selected_key === answer.key && !submission.is_correct) classes.push("incorrect");
    }
    label.className = classes.join(" ");
    label.innerHTML = `
      <input type="radio" name="answer" value="${answer.key}" ${selectedKey === answer.key ? "checked" : ""} ${
      submission ? "disabled" : ""
    } />
      <span class="answer-key">${answer.key}</span>
      <span class="answer-copy">
        <span>${escapeHtml(answer.text)}</span>
        ${
          showTranslation && answer.translation
            ? `<span class="answer-translation">${escapeHtml(answer.translation)}${
                answer.translationSource === "ai" ? ' <span class="translation-tag">AI generated</span>' : ""
              }</span>`
            : showTranslation
              ? `<span class="answer-translation missing">Translation unavailable</span>`
              : ""
        }
      </span>
    `;
    el.answerForm.appendChild(label);
  });
}

function isMobileDrawer() {
  return window.matchMedia("(max-width: 768px)").matches;
}

function closeHintSidebar() {
  const question = currentQuestion();
  if (!question) return;
  const ui = uiForQuestion(question.id);
  ui.hintOpen = false;
  el.hintToggle.checked = false;
  saveState();
  renderQuestion();
}

function renderQuestionTranslation(question, showTranslation) {
  let translation = document.querySelector(".question-translation");
  if (!showTranslation) {
    if (translation) translation.remove();
    return;
  }

  if (!translation) {
    translation = document.createElement("p");
    translation.className = "question-translation";
    el.questionText.insertAdjacentElement("afterend", translation);
  }

  translation.textContent = question.text_translation;
}

function renderHintSidebar(aiError = null) {
  const question = currentQuestion();
  if (!question) return;
  const ui = uiForQuestion(question.id);
  const hint = state.activeQuestionHint;
  const showSidebar = canUseAiFeatures() && ui.hintOpen;
  const mobileDrawer = isMobileDrawer();

  el.examLayout.classList.toggle("with-sidebar", showSidebar && !mobileDrawer);
  el.hintBackdrop.classList.toggle("hidden", !(showSidebar && mobileDrawer));
  el.hintSidebar.classList.toggle("drawer-open", showSidebar && mobileDrawer);
  if (mobileDrawer) {
    el.hintSidebar.classList.remove("hidden");
    document.body.classList.toggle("hint-drawer-open", showSidebar);
  } else {
    el.hintSidebar.classList.toggle("hidden", !showSidebar);
    el.hintSidebar.classList.remove("drawer-open");
    document.body.classList.remove("hint-drawer-open");
  }

  if (!showSidebar) return;

  if (aiError) {
    el.hintTitle.textContent = "OpenAI error";
    el.hintText.textContent = aiError;
    el.memoryTrick.textContent = "Fix the API issue to re-enable hints.";
    return;
  }

  if (!hint) {
    el.hintTitle.textContent = "Loading hint...";
    el.hintText.textContent = "Preparing a simple English + German explanation for this question.";
    el.memoryTrick.textContent = "This will appear when the hint response is ready.";
    return;
  }

  el.hintTitle.textContent = hint.hintTitle;
  el.hintText.textContent = hint.hintText;
  el.memoryTrick.textContent = hint.memoryTrick;
}

function renderFeedback(submission, aiError = null) {
  el.feedbackBox.classList.toggle("hidden", !submission);
  if (!submission) {
    el.feedbackBox.innerHTML = "";
    el.feedbackBox.className = "feedback-box hidden";
    return;
  }

  el.feedbackBox.className = `feedback-box ${submission.is_correct ? "correct" : "incorrect"}`;
  if (!canUseAiFeatures()) {
    el.feedbackBox.innerHTML = `
      <p class="feedback-status">${submission.is_correct ? "Correct" : "Incorrect"}</p>
      <p><strong>Correct answer:</strong> ${escapeHtml(submission.correct_answer_text_original)}</p>
    `;
    return;
  }

  if (aiError) {
    el.feedbackBox.innerHTML = `
      <p class="feedback-status">${submission.is_correct ? "Correct" : "Incorrect"}</p>
      <p><strong>Correct answer:</strong> ${escapeHtml(submission.correct_answer_text_original)}</p>
      <p><strong>OpenAI error:</strong> ${escapeHtml(aiError)}</p>
    `;
    return;
  }

  el.feedbackBox.innerHTML = `
    <p class="feedback-status">${submission.is_correct ? "Correct" : "Incorrect"}</p>
    <p><strong>Correct answer:</strong> ${escapeHtml(submission.correct_answer_text_original)}</p>
    <p>${escapeHtml(submission.feedback.feedbackText)}</p>
    <p><strong>Memory trick:</strong> ${escapeHtml(submission.feedback.memoryTrick)}</p>
  `;
}

async function ensureHintForCurrentQuestion() {
  const question = currentQuestion();
  if (!canUseAiFeatures() || !question || state.hintCache[question.id]) return;

  state.hintLoadingQuestionId = question.id;
  renderQuestion();
  renderHintSidebar();
  try {
    const hintResponse = await fetchJson("/api/exam/hint", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        exam_id: state.exam.exam_id,
        question_id: question.id,
        profile_id: state.profileId,
      }),
    });
    state.aiErrorByQuestion[question.id] = hintResponse.ai_error || null;
    if (hintResponse.hint) {
      state.hintCache[question.id] = hintResponse.hint;
      state.hintSourceByQuestion[question.id] = hintResponse.source;
      state.activeQuestionHint = hintResponse.hint;
    } else {
      state.activeQuestionHint = null;
    }
    saveState();
  } catch (error) {
    state.aiErrorByQuestion[question.id] = error.message;
    el.hintTitle.textContent = "OpenAI error";
    el.hintText.textContent = error.message;
    el.memoryTrick.textContent = "Fix the API issue to re-enable hints.";
  } finally {
    state.hintLoadingQuestionId = null;
    renderQuestion();
  }
}

async function ensureGeneratedTranslationsForCurrentQuestion() {
  const question = currentQuestion();
  if (!question || !canUseAiFeatures()) return;

  const hasMissingDatasetTranslations = question.answers.some((answer) => !answer.text_translation);
  if (!hasMissingDatasetTranslations || state.generatedTranslationsByQuestion[question.id]) return;

  try {
    const response = await fetchJson("/api/exam/translation", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        exam_id: state.exam.exam_id,
        question_id: question.id,
        profile_id: state.profileId,
      }),
    });

    if (response.ai_error) {
      state.aiErrorByQuestion[question.id] = response.ai_error;
      return;
    }

    state.generatedTranslationsByQuestion[question.id] = Object.fromEntries(
      (response.generated_answers || []).map((item) => [item.key, item.text])
    );
    saveState();
  } catch (error) {
    state.aiErrorByQuestion[question.id] = error.message;
  }
}

async function submitCurrentAnswer() {
  const question = currentQuestion();
  const selectedKey = getSelectedAnswerFromForm();
  if (!question || !selectedKey) {
    window.alert("Please select one answer before submitting.");
    return;
  }

  setSubmitLoading(true);
  try {
    const response = await fetchJson("/api/exam/answer", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        exam_id: state.exam.exam_id,
        question_id: question.id,
        selected_key: selectedKey,
        profile_id: state.profileId,
      }),
    });

    state.submissions[question.id] = response;
    state.datasetProgress = {
      correct: response.dataset_correct_answers ?? state.datasetProgress.correct ?? 0,
      total: response.total_dataset_questions ?? state.datasetProgress.total ?? 0,
    };
    renderDatasetProgress();
    state.aiErrorByQuestion[question.id] = response.ai_error || null;
    if (canUseAiFeatures()) {
      if (response.feedback) {
        state.hintCache[question.id] = response.feedback;
        state.hintSourceByQuestion[question.id] = response.feedback_source;
        state.activeQuestionHint = response.feedback;
      } else {
        state.activeQuestionHint = null;
        delete state.hintCache[question.id];
        delete state.hintSourceByQuestion[question.id];
      }
    } else {
      state.activeQuestionHint = null;
    }
    saveState();
    renderQuestion();

    if (response.is_exam_complete) {
      await renderResultScreen();
    }
  } catch (error) {
    el.feedbackBox.className = "feedback-box incorrect";
    el.feedbackBox.classList.remove("hidden");
    el.feedbackBox.innerHTML = `
      <p class="feedback-status">Submit failed</p>
      <p>${escapeHtml(error.message || "Request failed")}</p>
    `;
  } finally {
    setSubmitLoading(false);
    renderQuestion();
  }
}

function goToNextQuestion() {
  if (state.currentIndex < state.exam.questions.length - 1) {
    state.currentIndex += 1;
    saveState();
    renderQuestion();
  }
}

function goToPreviousQuestion() {
  if (state.currentIndex > 0 && !isSubmitting()) {
    state.currentIndex -= 1;
    saveState();
    renderQuestion();
  }
}

async function renderResultScreen() {
  showPanel("result");
  const result = await fetchJson(
    `/api/exam/result/${encodeURIComponent(state.exam.exam_id)}?profile_id=${encodeURIComponent(state.profileId)}`
  );

  el.resultHeadline.textContent = `${result.score} / ${result.total_questions}`;
  el.resultCopy.textContent = result.passed
    ? `Passed. You reached the exam threshold of ${result.passing_score}.`
    : `Not passed yet. You need ${result.passing_score} correct answers.`;

  const resultTone = result.passed
    ? "passed"
    : result.score >= result.passing_score - 3
      ? "close"
      : "failed";
  el.resultScreen.dataset.tone = resultTone;
  state.datasetProgress = {
    correct: result.dataset_correct_answers ?? state.datasetProgress.correct ?? 0,
    total: result.total_dataset_questions ?? state.datasetProgress.total ?? 0,
  };
  el.resultSummary.innerHTML = `
    <p><strong>Status:</strong> ${result.passed ? "Passed" : "Not passed"}</p>
    <p><strong>Answered:</strong> ${result.answered_count} / ${result.total_questions}</p>
    <p><strong>Mastered:</strong> ${result.dataset_correct_answers} / ${result.total_dataset_questions}</p>
    <p><strong>Profile:</strong> ${escapeHtml(result.profile_id)}</p>
  `;

  renderResultReview(result.incorrect_questions);
  saveState();
}

async function openHistoryScreen() {
  if (!state.profileId) return;
  const response = await fetchJson(`/api/profile/review?profile_id=${encodeURIComponent(state.profileId)}`);
  state.historyReview = {
    correct: response.correct_questions || [],
    incorrect: response.incorrect_questions || [],
  };
  showPanel("history");
  renderHistoryReview();
  saveState();
}

function goBackFromHistory() {
  if (state.exam) {
    if (isExamFinishedLocally()) {
      showPanel("result");
      return;
    }

    renderExamScreen();
    return;
  }

  showPanel("start");
}

function setHistoryFilter(filter) {
  state.historyFilter = filter;
  renderHistoryReview();
  saveState();
}

function renderDatasetProgress() {
  const correct = Number(state.datasetProgress.correct || 0);
  const total = Number(state.datasetProgress.total || 0);
  const percent = total > 0 ? Math.max(0, Math.min(100, (correct / total) * 100)) : 0;
  let tone = "progress-low";
  if (percent >= 70) {
    tone = "progress-high";
  } else if (percent >= 35) {
    tone = "progress-mid";
  }

  el.datasetProgressText.textContent = `${correct} / ${total}`;
  el.datasetProgressFill.style.width = `${percent}%`;
  el.datasetProgressFill.classList.remove("progress-low", "progress-mid", "progress-high");
  el.datasetProgressFill.classList.add(tone);
}

function renderResultReview(items) {
  if (!items.length) {
    el.reviewList.innerHTML = '<div class="review-item"><h3>No incorrect answers</h3><p>Clean run.</p></div>';
    return;
  }

  el.reviewList.innerHTML = items
    .map(
      (item) => `
      <article class="review-item">
        <h3>Frage ${item.question_id}</h3>
        <p><strong>German:</strong> ${escapeHtml(item.text_original)}</p>
        <p><strong>You chose:</strong> ${escapeHtml(item.selected_key)}. ${escapeHtml(item.selected_answer_text_original)}</p>
        <p><strong>Result:</strong> ${item.is_correct ? "Correct" : "Incorrect"}</p>
        <p><strong>Correct:</strong> ${escapeHtml(item.correct_key)}. ${escapeHtml(item.correct_answer_text_original)}</p>
        ${
          canUseAiFeatures()
            ? item.ai_error
              ? `<p><strong>OpenAI error:</strong> ${escapeHtml(item.ai_error)}</p>`
              : `<p>${escapeHtml(item.feedback_text)}</p><p><strong>Memory trick:</strong> ${escapeHtml(item.memory_trick)}</p>`
            : ""
        }
      </article>
    `
    )
    .join("");
}

function renderHistoryReview() {
  const items = state.historyFilter === "correct" ? state.historyReview.correct : state.historyReview.incorrect;
  el.historyCorrectBtn.classList.toggle("active", state.historyFilter === "correct");
  el.historyIncorrectBtn.classList.toggle("active", state.historyFilter === "incorrect");
  const noMistakes = state.historyReview.incorrect.length === 0;
  el.retryMistakesBtn.classList.toggle("hidden", noMistakes);
  el.retryMistakesBtnBottom.classList.toggle("hidden", noMistakes);

  if (!items.length) {
    el.historyList.innerHTML = state.historyFilter === "correct"
      ? '<div class="review-item"><h3>No completed answers yet</h3><p>Completed questions across exams will appear here.</p></div>'
      : '<div class="review-item"><h3>No mistakes yet</h3><p>Mistakes across exams will appear here.</p></div>';
    return;
  }

  el.historyList.innerHTML = items
    .map(
      (item) => `
      <article class="review-item">
        <h3>Frage ${item.question_id}</h3>
        <p><strong>German:</strong> ${escapeHtml(item.text_original)}</p>
        <p><strong>You chose:</strong> ${escapeHtml(item.selected_key)}. ${escapeHtml(item.selected_answer_text_original)}</p>
        <p><strong>Result:</strong> ${item.is_correct ? "Correct" : "Incorrect"}</p>
        <p><strong>Correct:</strong> ${escapeHtml(item.correct_key)}. ${escapeHtml(item.correct_answer_text_original)}</p>
        ${
          canUseAiFeatures()
            ? item.ai_error
              ? `<p><strong>OpenAI error:</strong> ${escapeHtml(item.ai_error)}</p>`
              : `<p>${escapeHtml(item.feedback_text)}</p><p><strong>Memory trick:</strong> ${escapeHtml(item.memory_trick)}</p>`
            : ""
        }
      </article>
    `
    )
    .join("");
}

async function startPractice() {
  // If history was cached before the backend added answers[], refetch it now.
  const needsRefresh = state.historyReview.incorrect.some((q) => !q.answers?.length);
  if (needsRefresh) {
    const response = await fetchJson(
      `/api/profile/review?profile_id=${encodeURIComponent(state.profileId)}`
    );
    state.historyReview = {
      correct: response.correct_questions || [],
      incorrect: response.incorrect_questions || [],
    };
    renderHistoryReview();
    saveState();
  }

  const mistakes = state.historyReview.incorrect;
  if (!mistakes.length) return;
  const shuffled = [...mistakes].sort(() => Math.random() - 0.5);
  state.practice = { questions: shuffled, index: 0, answers: {} };
  showPanel("practice");
  renderPracticeQuestion();
}

function renderPracticeQuestion() {
  const { questions, index, answers } = state.practice;
  const item = questions[index];
  if (!item) return;

  el.practiceQuestionArea.classList.remove("hidden");
  el.practiceCompleteArea.classList.add("hidden");

  el.practiceProgressLabel.textContent = `Question ${index + 1} / ${questions.length}`;
  el.practiceQuestionText.textContent = item.text_original;

  const answer = answers[item.question_id];

  el.practiceAnswerForm.innerHTML = "";
  (item.answers || []).forEach((opt) => {
    const label = document.createElement("label");
    const classes = ["answer-option"];
    if (answer) {
      if (opt.key === item.correct_key) classes.push("correct");
      else if (opt.key === answer.selectedKey) classes.push("incorrect");
    }
    label.className = classes.join(" ");
    label.innerHTML = `
      <input type="radio" name="practice-answer" value="${opt.key}"
        ${answer?.selectedKey === opt.key ? "checked" : ""}
        ${answer ? "disabled" : ""} />
      <span class="answer-key">${opt.key}</span>
      <span class="answer-copy"><span>${escapeHtml(opt.text_original)}</span></span>
    `;
    el.practiceAnswerForm.appendChild(label);
  });

  el.practiceSubmitBtn.classList.toggle("hidden", Boolean(answer));
  el.practiceNextBtn.classList.toggle("hidden", !answer);
  const isLast = index === questions.length - 1;
  el.practiceNextBtn.textContent = isLast ? "See Results" : "Next Question";

  if (answer) {
    el.practiceFeedbackBox.className = `feedback-box ${answer.isCorrect ? "correct" : "incorrect"}`;
    el.practiceFeedbackBox.classList.remove("hidden");
    el.practiceFeedbackBox.innerHTML = `
      <p class="feedback-status">${answer.isCorrect ? "Correct" : "Incorrect"}</p>
      <p><strong>Correct answer:</strong> ${escapeHtml(item.correct_answer_text_original)}</p>
      ${item.feedback_text ? `<p>${escapeHtml(item.feedback_text)}</p>` : ""}
      ${item.memory_trick ? `<p><strong>Memory trick:</strong> ${escapeHtml(item.memory_trick)}</p>` : ""}
    `;
  } else {
    el.practiceFeedbackBox.className = "feedback-box hidden";
    el.practiceFeedbackBox.innerHTML = "";
  }
}

function submitPracticeAnswer() {
  const { questions, index } = state.practice;
  const item = questions[index];
  const selected = el.practiceAnswerForm.querySelector('input[name="practice-answer"]:checked');
  if (!selected) {
    window.alert("Please select an answer.");
    return;
  }
  const selectedKey = selected.value;
  state.practice.answers[item.question_id] = {
    selectedKey,
    isCorrect: selectedKey === item.correct_key,
  };
  renderPracticeQuestion();
}

function nextPracticeQuestion() {
  const { questions, index } = state.practice;
  if (index < questions.length - 1) {
    state.practice.index += 1;
    renderPracticeQuestion();
  } else {
    renderPracticeComplete();
  }
}

function renderPracticeComplete() {
  el.practiceQuestionArea.classList.add("hidden");
  el.practiceCompleteArea.classList.remove("hidden");

  const { questions, answers } = state.practice;
  const correct = Object.values(answers).filter((a) => a.isCorrect).length;
  const total = questions.length;
  const tone = correct === total ? "passed" : correct >= Math.ceil(total / 2) ? "close" : "failed";

  el.practiceScoreHeadline.textContent = `${correct} / ${total}`;
  el.practiceScoreHeadline.className = `result-${tone}`;
  el.practiceScoreCopy.textContent =
    correct === total
      ? "Perfect run. All mistakes cleared!"
      : `${total - correct} question${total - correct !== 1 ? "s" : ""} still need work.`;
}

function currentQuestion() {
  return state.exam?.questions?.[state.currentIndex] || null;
}

function uiForQuestion(questionId) {
  if (!state.uiByQuestion[questionId]) {
    state.uiByQuestion[questionId] = {
      translationOpen: false,
      emojiEnabled: false,
      hintOpen: false,
    };
  }
  return state.uiByQuestion[questionId];
}

function persistUiState() {
  const question = currentQuestion();
  if (!question) return;
  state.uiByQuestion[question.id] = {
    translationOpen: el.translationToggle.checked,
    emojiEnabled: canUseAiFeatures() && el.emojiToggle.checked,
    hintOpen: canUseAiFeatures() && el.hintToggle.checked,
  };
  saveState();
}

function getSelectedAnswerFromForm() {
  const selected = el.answerForm.querySelector('input[name="answer"]:checked');
  return selected ? selected.value : null;
}

function isExamFinishedLocally() {
  return state.exam && Object.keys(state.submissions).length >= state.exam.total_questions;
}

function showPanel(name) {
  el.startScreen.classList.toggle("active", name === "start");
  el.examScreen.classList.toggle("active", name === "exam");
  el.resultScreen.classList.toggle("active", name === "result");
  el.historyScreen.classList.toggle("active", name === "history");
  el.practiceScreen.classList.toggle("active", name === "practice");

  if (name !== "exam") {
    document.body.classList.remove("hint-drawer-open");
    el.hintBackdrop.classList.add("hidden");
    el.hintSidebar.classList.remove("drawer-open");
  }
}

function setSubmitLoading(loading) {
  el.submitBtn.dataset.loading = loading ? "true" : "false";
  const label = el.submitBtn.querySelector(".btn-label");
  const spinner = el.submitBtn.querySelector(".btn-spinner");
  if (label) {
    label.textContent = loading ? "Submitting..." : "Submit Answer";
  }
  if (spinner) {
    spinner.classList.toggle("hidden", !loading);
  }
  el.submitBtn.disabled = loading;
  el.nextBtn.disabled = loading;
  el.backBtn.disabled = loading || state.currentIndex === 0;
}

function isSubmitting() {
  return el.submitBtn.dataset.loading === "true";
}

function saveState() {
  const snapshot = {
    profileId: state.profileId,
    aiStatus: state.aiStatus,
    exam: state.exam,
    datasetProgress: state.datasetProgress,
    historyFilter: state.historyFilter,
    historyReview: state.historyReview,
    currentIndex: state.currentIndex,
    submissions: state.submissions,
    hintCache: state.hintCache,
    hintSourceByQuestion: state.hintSourceByQuestion,
    aiErrorByQuestion: state.aiErrorByQuestion,
    generatedTranslationsByQuestion: state.generatedTranslationsByQuestion,
    uiByQuestion: state.uiByQuestion,
  };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(snapshot));
}

function restoreState() {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return;

  try {
    const snapshot = JSON.parse(raw);
    state.profileId = snapshot.profileId || null;
    state.aiStatus = snapshot.aiStatus || null;
    state.exam = snapshot.exam || null;
    state.datasetProgress = snapshot.datasetProgress || { correct: 0, total: 0 };
    state.historyFilter = snapshot.historyFilter || "incorrect";
    state.historyReview = snapshot.historyReview || { correct: [], incorrect: [] };
    state.currentIndex = snapshot.currentIndex || 0;
    state.submissions = snapshot.submissions || {};
    state.hintCache = snapshot.hintCache || {};
    state.hintSourceByQuestion = snapshot.hintSourceByQuestion || {};
    state.aiErrorByQuestion = snapshot.aiErrorByQuestion || {};
    state.generatedTranslationsByQuestion = snapshot.generatedTranslationsByQuestion || {};
    state.uiByQuestion = snapshot.uiByQuestion || {};
  } catch {
    localStorage.removeItem(STORAGE_KEY);
  }
}

async function fetchJson(url, options = {}) {
  const response = await fetch(url, options);
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data.error || "Request failed");
  }
  return data;
}

function canUseAiFeatures() {
  return Boolean(state.aiStatus?.enabled);
}

function updateAiBadges(source = null, aiError = null, loading = false) {
  let text = "AI: disabled";
  if (state.aiStatus?.enabled) {
    const model = state.aiStatus.model || "configured";
    if (loading) {
      text = `AI: loading (${model})`;
    } else if (aiError) {
      text = "AI: error";
    } else if (source === "openai") {
      text = `AI: OpenAI (${model})`;
    } else {
      text = `AI: enabled (${model})`;
    }
  }

  el.aiBadgeText.textContent = text;
  el.aiBadgeSpinner.classList.toggle("hidden", !loading);
  el.startAiBadge.textContent = text;
}

function escapeHtml(text) {
  return String(text)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}
