# F16 — Explanation Generator

**Phase:** 3 — AI
**Size:** S (one session)
**Branch:** `feat/P3-explanations`
**Depends on:** F15, F08

## Objective

Use local LLM to generate human-readable explanations for recommendations, enriching the raw rule output with context.

## Deliverables

### 1. ExplanationService (`lso-ai/src/explanation.rs`)
```rust
pub struct ExplanationService {
    llm: Box<dyn LlmProvider>,
}

impl ExplanationService {
    /// Generate explanation for a recommendation with system context
    pub async fn explain(&self, rec: &Recommendation, probe_data: &[ProbeResult]) -> Result<String, AiError>;

    /// Batch explain multiple recommendations
    pub async fn explain_batch(&self, recs: &[Recommendation], data: &[ProbeResult]) -> Vec<Result<String, AiError>>;
}
```

### 2. Context Builder
- Gather relevant probe data for the recommendation
- Build context string with key metrics
- Keep within model context window (< 2048 tokens input)

### 3. UI Integration
- "AI Explanation" expandable section on each recommendation card
- Loading indicator during generation
- "Regenerate" button
- Fallback text when AI unavailable

### 4. Caching
- Cache explanations in DB keyed by recommendation ID + data hash
- Invalidate when underlying data changes significantly

## Acceptance Criteria

- [ ] Explanations are non-technical, clear, under 3 sentences
- [ ] Context from probe data included in prompt
- [ ] Cached — same recommendation doesn't re-generate
- [ ] Streaming display in UI
- [ ] Fallback to rule description text when AI offline
- [ ] LLM output is NEVER used as input to the actuator
