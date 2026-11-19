-- 0018_harness_session_limits.sql
-- Persist per-session `cost_limit_usd` and `max_steps` on `harness_sessions`
-- so `POST /v1/harness/sessions/{id}/interrupt/answer` (HITL `ask_user`) and
-- `POST /v1/harness/sessions/{id}:resume` preserve budget and step caps.

ALTER TABLE harness_sessions
  ADD COLUMN IF NOT EXISTS cost_limit_usd DOUBLE PRECISION,
  ADD COLUMN IF NOT EXISTS max_steps INTEGER;
