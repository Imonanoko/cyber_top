-- Map definitions
CREATE TABLE IF NOT EXISTS maps (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    arena_radius REAL NOT NULL DEFAULT 12.0,
    placements_json TEXT NOT NULL DEFAULT '[]'
);
