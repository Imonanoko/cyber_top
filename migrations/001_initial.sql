-- Initial schema for Cyber Top

CREATE TABLE IF NOT EXISTS tops (
    id TEXT PRIMARY KEY,
    base_stats_json TEXT NOT NULL,
    skin_id TEXT NOT NULL,
    balance_version INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS parts (
    id TEXT PRIMARY KEY,
    slot TEXT NOT NULL,
    kind TEXT NOT NULL,
    spec_json TEXT NOT NULL,
    balance_version INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS builds (
    id TEXT PRIMARY KEY,
    top_id TEXT NOT NULL,
    weapon_id TEXT NOT NULL,
    shaft_id TEXT NOT NULL,
    chassis_id TEXT NOT NULL,
    screw_id TEXT NOT NULL,
    note TEXT
);

CREATE TABLE IF NOT EXISTS effective_cache (
    build_id TEXT PRIMARY KEY,
    effective_stats_json TEXT NOT NULL,
    computed_at INTEGER NOT NULL,
    balance_version INTEGER NOT NULL,
    hash TEXT NOT NULL
);
