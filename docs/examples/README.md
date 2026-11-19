# Yanshi Examples Documentation

This directory documents runnable tutorials and example walkthroughs for Yanshi's
Content Studio generators, deterministic DAG workflows, and agent-native loops.

## Start Here

- **[Content Studio Catalogue (`STUDIO_CATALOGUE.md`)](../STUDIO_CATALOGUE.md)** —
  Complete guide to the 19 built-in deliverables under
  [`examples/generators/`](../../examples/generators/) (`illustrated_slides`,
  `tech_slides_video`, `slides`, `video_overview`, `audio_overview`, `report`,
  `data_table`, `infographic`, `mindmap`, `circuit_diagram`, `flashcards`,
  `quiz`, `repo_architecture`, `backend_project_planner`, `paper_reader`,
  `video_summarizer`, `episodic_drama_studio`, `debate_podcast_show`,
  `worldbuilding_bible`) plus Web Studio and the `Yanshi Studio` desktop app.
- **[SDK Example Matrix (`examples/README.md`)](../../examples/README.md)** —
  Canonical capability-to-example table across all workspace crates.
- **[End-to-End Applications (`examples/applications/README.md`)](../../examples/applications/README.md)** —
  Product-shaped dogfooding applications (`content-studio`,
  `rust-traits-tutorial`, `doc-translator`, `podcast-mastering`, `code-reviewer`).

---

## Step-by-Step Tutorials in This Manual

### 📘 [Runnable Tutorials (中文)](./runnable_tutorials_zh.md)

Copy-and-run tutorials for the core Yanshi execution paths:

- Fixed DAG workflow (`yanshi-core/examples/fixed_dag_workflow.rs`)
- Agent-native ReAct runtime (`yanshi-agents/examples/agent_native_react.rs`)
- DAG + Agent hybrid workflow (`yanshi-agents/examples/hybrid_workflow_agent.rs`)
- Skill + MCP tool discovery and call (`yanshi-skills/examples/skill_calls_mcp_tool.rs`)
- `WorkflowTool` wrapping a child DAG

### 🧰 [CLI Config-First Tutorial](./cli_config_first_tutorial.md)

Copy-and-run CLI walkthrough for the V2 YAML workflow and Skill paths:

- Offline mock model configuration
- Fixed DAG validation, `--dry-run`, and execution
- Skill `inspect` / `list-tools` / `test` / `run` with `--model`, `--memory`, and `--trace`
- `skill_agent` hybrid workflow execution
- RAG + Skill workflow validation (`yanshi-cli --features rag`)
- Signed marketplace package installation

### 🦀 [Rust Interview Code-First Workflow (中文)](./rust_interview_code_first_proper_test_zh.md)

End-to-end walkthrough of authoring and testing a typed Rust SDK workflow.

### 🤖 [Simple Agent LLM Flow](./simple_agent_llm_flow.md)

Walkthrough and Mermaid sequence/structure diagrams
([`simple_agent_llm_flow_diagram.mermaid`](./simple_agent_llm_flow_diagram.mermaid),
[`execution_flow_diagram.mermaid`](./execution_flow_diagram.mermaid)) for
conditional LLM routing and observability.

---

## Validating & Smoke-Testing Examples Locally

```bash
# Type-check every crate example across the workspace
cargo check --workspace --examples

# Strictly validate any Studio generator or CLI workflow YAML
cargo run -p yanshi-cli -- workflow validate examples/generators/illustrated_slides.yml --strict

# Run the offline CI example smoke suite
cargo xtask examples-smoke
```
