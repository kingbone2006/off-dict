CREATE TABLE history  (headword TEXT, lang TEXT, looked_up_at INTEGER);
CREATE TABLE favorite (headword TEXT, lang TEXT, folder TEXT, note TEXT, created_at INTEGER);
CREATE TABLE srs_card (headword TEXT, lang TEXT, stability REAL, difficulty REAL,
                       due_at INTEGER, reps INTEGER, lapses INTEGER);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);
CREATE INDEX idx_history_time ON history(looked_up_at DESC);
CREATE INDEX idx_history_word ON history(lang, headword);
CREATE UNIQUE INDEX idx_favorite_word ON favorite(lang, headword);
