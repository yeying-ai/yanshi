-- AA3 (docs/RFC_CONTENT_STUDIO.md §6): persist the generator's structured
-- `Deliverable` JSON (title, content, citations, files) alongside notebook
-- artifact links so `GET /v1/notebooks/{id}/artifacts` exposes citations
-- directly without replaying run event history.

ALTER TABLE notebook_artifacts
  ADD COLUMN IF NOT EXISTS deliverable JSONB;
