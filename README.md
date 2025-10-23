# 🦀 Ragent  
> _A Rust-native framework for autonomous reasoning — where models become agents, and LLMs act as persons. Type-safe, checkpointed, and observable._

---

### ✨ Overview

**Ragent** is a modern **agent framework built in Rust**,  
inspired by [Claude Code](https://claude.ai), [Codex](https://openai.com/blog/openai-codex), and [LangGraph](https://github.com/langchain-ai/langgraph).  

It’s not a chain or a workflow engine — it’s a **thinking framework**.  
Ragent reimagines agent architecture for Rust’s precision and determinism,  
focusing on reasoning continuity, observability, and identity.

> In Ragent, **the model is the agent**, and the agent behaves like a **person** —  
> capable of thinking, remembering, and recovering.

---

### 🧩 Core Philosophy

1. **👤 Treat LLM as Person**  
   The model is not a callable API. It’s a collaborator with intent, personality, and context —  
   capable of planning, acting, observing, and reflecting in natural loops.

2. **🧠 Model as Agent**  
   Ragent doesn’t orchestrate models as functions.  
   It provides a runtime where a model can **exist as an autonomous process** with persistent cognition and recoverable state.

3. **💾 Checkpointed Reasoning**  
   Every `thought`, `action`, and `observation` is durably stored.  
   Agents can **pause, resume, or replay** reasoning trajectories deterministically.

4. **🔍 Observability by Design**  
   Reasoning is no longer opaque.  
   Every step is traceable through structured telemetry, enabling deep debugging and performance insight.

---

### ⚙️ Features

- 🧱 **Type-safe primitives** for reasoning and tools  
- 💾 **Checkpoint & Resume** for persistent state and deterministic recovery  
- 🔁 **Replay & Compare** agent runs with identical seeds  
- 🔍 **Built-in Observability** via structured tracing and metrics  
- 🧩 **Composable Agents** — minimal, modular, and testable  
- ⚙️ **Rust-first architecture** — precise, concurrent, and reliable  

---

### 🧱 Architectural Vision

Ragent positions itself as the **reasoning runtime layer** for Rust-based AI systems —  
bridging **language intelligence** with **systems engineering**.  

It turns “thinking” into an **auditable, reproducible process**,  
where reasoning can be versioned, resumed, and studied — like a running system rather than a one-off API call.

---

### 🧰 Inspirations

- [Claude Code](https://claude.ai) — human-style reasoning and collaboration  
- [Codex](https://openai.com/blog/openai-codex) — intelligence expressed through code  
- [LangGraph](https://github.com/langchain-ai/langgraph) — structured orchestration of stateful reasoning  

---

### 📜 License

MIT © 2025 Morris Liu

---

> _Ragent — Think in loops. Reason in public. Model as agent._
