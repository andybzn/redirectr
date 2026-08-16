CREATE TABLE links (
    code TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT (datetime('now', 'utc')) NOT NULL
);