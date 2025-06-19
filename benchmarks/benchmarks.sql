-- SQLite DB for benchmark tracking
-- Table: benchmarks
CREATE TABLE IF NOT EXISTS benchmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    commit_sha TEXT NOT NULL,
    branch TEXT NOT NULL,
    metric_category TEXT NOT NULL,
    metric_name TEXT NOT NULL,
    value REAL NOT NULL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Table: runs
CREATE TABLE IF NOT EXISTS runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    commit_sha TEXT NOT NULL,
    branch TEXT NOT NULL,
    run_time DATETIME DEFAULT CURRENT_TIMESTAMP
);
