-- IAD PostgreSQL schema

CREATE TABLE IF NOT EXISTS agents (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL UNIQUE,
    version       TEXT NOT NULL,
    status        TEXT NOT NULL DEFAULT 'registered',
    pid           BIGINT,
    manifest      JSONB NOT NULL DEFAULT '{}',
    generation_id BIGINT NOT NULL DEFAULT 0,
    failure_count BIGINT NOT NULL DEFAULT 0,
    started_at    TIMESTAMPTZ,
    stopped_at    TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS agent_incidents (
    id            TEXT PRIMARY KEY,
    agent_id      TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    agent_name    TEXT NOT NULL,
    generation_id BIGINT NOT NULL,
    exit_code     INTEGER,
    signal        INTEGER,
    reason        TEXT NOT NULL,
    occurred_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_incidents_agent ON agent_incidents(agent_name, occurred_at DESC);

CREATE TABLE IF NOT EXISTS tasks (
    id             TEXT PRIMARY KEY,
    title          TEXT,
    payload        JSONB NOT NULL DEFAULT '{}',
    capabilities   JSONB NOT NULL DEFAULT '[]',
    priority       INTEGER NOT NULL DEFAULT 3,
    status         TEXT NOT NULL DEFAULT 'pending',
    assigned_agent TEXT,
    result         JSONB,
    error          TEXT,
    retry_count    INTEGER NOT NULL DEFAULT 0,
    max_retries    INTEGER NOT NULL DEFAULT 3,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status, priority, created_at);

CREATE TABLE IF NOT EXISTS events (
    id         TEXT PRIMARY KEY,
    topic      TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload    JSONB NOT NULL DEFAULT '{}',
    source     TEXT NOT NULL,
    sequence   BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_events_topic ON events(topic, sequence);

CREATE TABLE IF NOT EXISTS agent_usage (
    id          TEXT PRIMARY KEY,
    task_id     TEXT NOT NULL,
    agent_name  TEXT NOT NULL,
    tokens_in   BIGINT NOT NULL DEFAULT 0,
    tokens_out  BIGINT NOT NULL DEFAULT 0,
    cost_usd    DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS daemon_nodes (
    id           TEXT PRIMARY KEY,
    grpc_addr    TEXT NOT NULL,
    rest_addr    TEXT NOT NULL,
    hostname     TEXT NOT NULL,
    version      TEXT NOT NULL,
    agents       INTEGER NOT NULL DEFAULT 0,
    tasks_pending INTEGER NOT NULL DEFAULT 0,
    healthy      BOOLEAN NOT NULL DEFAULT TRUE,
    last_seen    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
