-- AG2: Persist run resume context (`model`, `cost_limit_usd`, `notebook_id`,
-- `notebook_kind`) so `POST /v1/runs/{id}:resume` preserves LLM model
-- overrides, spend ceilings, and Notebook deliverable attachment; and persist
-- `raw_content` on `sources` so inline `text` note edits never duplicate
-- paragraph-chunker overlap boundaries.

ALTER TABLE runs
  ADD COLUMN IF NOT EXISTS model TEXT,
  ADD COLUMN IF NOT EXISTS cost_limit_usd DOUBLE PRECISION,
  ADD COLUMN IF NOT EXISTS notebook_id UUID REFERENCES notebooks(id) ON DELETE SET NULL,
  ADD COLUMN IF NOT EXISTS notebook_kind TEXT;

ALTER TABLE sources
  ADD COLUMN IF NOT EXISTS raw_content TEXT;
