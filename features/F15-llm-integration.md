# F15 — llama.cpp Integration Layer

**Phase:** 3 — AI
**Size:** M (1-2 sessions)
**Branch:** `feat/P3-llm-integration`
**Depends on:** F03

## Objective

Integrate local LLM inference via llama.cpp. This is the foundation for AI-powered explanations and file classification. Feature is optional — app works without it.

## Approach Decision

| Option | Pros | Cons |
|---|---|---|
| **ollama (subprocess)** | Easy setup, model management built-in, auto GPU detection | External dependency, must be pre-installed |
| **llama-cpp-rs (FFI)** | No external deps, bundled with app, full control | Complex build, large binary, manual model management |
| **Recommended: ollama first** | Ship faster, switch to FFI later if needed | — |

## Deliverables

### 1. LLM Trait (`lso-ai/src/lib.rs`)
```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, prompt: &str, max_tokens: u32) -> Result<String, AiError>;
    async fn is_available(&self) -> bool;
    fn model_name(&self) -> &str;
}
```

### 2. Ollama Backend (`lso-ai/src/ollama.rs`)
- Connect to local ollama HTTP API (`http://localhost:11434`)
- Exception to "no network" rule: localhost only, user-initiated
- Model pull guidance in UI if not installed
- Streaming response for real-time display

### 3. Fallback: No-AI Mode
- If ollama not running, all AI features gracefully degrade
- Recommendations show raw rule text instead of AI explanation
- Feature toggle in settings

### 4. Prompt Templates (`lso-ai/src/prompts.rs`)
```rust
pub fn explanation_prompt(recommendation: &Recommendation, context: &str) -> String {
    format!(
        "You are a system administrator explaining an issue to a non-technical user.\n\
         System finding: {}\n\
         Technical details: {}\n\
         Explain what this means, why it matters, and what the recommended action does.\n\
         Keep it under 3 sentences. No jargon.",
        recommendation.title, context
    )
}
```

## Steps

1. Implement `LlmProvider` trait
2. Implement ollama HTTP client (reqwest, localhost only)
3. Create prompt templates for explanations
4. Add availability check on app startup
5. Build settings UI for AI toggle and model selection
6. Test: generate explanation for a disk usage recommendation

## Acceptance Criteria

- [ ] Connects to local ollama instance only
- [ ] Generates coherent explanation for a recommendation
- [ ] Streaming output displayed in UI
- [ ] Graceful degradation when ollama unavailable
- [ ] No network calls except localhost:11434
- [ ] Model name configurable in settings
- [ ] AI feature toggle works (on/off)
