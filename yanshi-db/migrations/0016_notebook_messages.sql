-- AF2 (docs/RFC_CONTENT_STUDIO.md §6): persist grounded `/ask` Q&A turns
-- per notebook so multi-turn research threads survive page reloads, Studio
-- navigation, and portable `export`/`import`.

CREATE TABLE IF NOT EXISTS notebook_messages (
  id                UUID PRIMARY KEY,
  notebook_id       UUID NOT NULL REFERENCES notebooks(id) ON DELETE CASCADE,
  tenant_id         TEXT NOT NULL,
  question          TEXT NOT NULL,
  answer            TEXT NOT NULL,
  model             TEXT NOT NULL,
  citations         JSONB NOT NULL DEFAULT '[]'::jsonb,
  used_chunk_count  INTEGER NOT NULL DEFAULT 0,
  created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS notebook_messages_notebook_idx
  ON notebook_messages (notebook_id, created_at ASC, id ASC);
