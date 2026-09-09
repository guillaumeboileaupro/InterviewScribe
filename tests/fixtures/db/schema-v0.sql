PRAGMA foreign_keys = ON;
PRAGMA user_version = 0;

CREATE TABLE interview (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    language TEXT,
    mode TEXT NOT NULL CHECK (mode IN ('posteriori','realtime')),
    audio_path TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('imported','transcribing','transcribed','error')),
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE speaker (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    interview_id INTEGER NOT NULL REFERENCES interview(id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    color TEXT NOT NULL,
    display_name TEXT
);

CREATE TABLE segment (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    interview_id INTEGER NOT NULL REFERENCES interview(id) ON DELETE CASCADE,
    speaker_id INTEGER REFERENCES speaker(id) ON DELETE SET NULL,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL,
    raw_text TEXT NOT NULL,
    confidence REAL,
    status TEXT NOT NULL DEFAULT 'raw' CHECK (status IN ('raw','uncertain'))
);

CREATE TABLE edit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    segment_id INTEGER NOT NULL REFERENCES segment(id) ON DELETE CASCADE,
    operation TEXT NOT NULL,
    before_text TEXT NOT NULL,
    after_text TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE setting (
    interview_id INTEGER PRIMARY KEY REFERENCES interview(id) ON DELETE CASCADE,
    show_timestamps INTEGER NOT NULL DEFAULT 1,
    model_path TEXT NOT NULL,
    language TEXT,
    cleanup_enabled INTEGER NOT NULL DEFAULT 0,
    device TEXT NOT NULL DEFAULT 'cpu'
);

INSERT INTO interview (
    id, title, language, mode, audio_path, status, error_message, created_at, updated_at
) VALUES (
    1, 'Projet synthetique N-1', 'fr', 'posteriori', '/fixture/audio.wav',
    'transcribed', NULL, '1700000000', '1700000001'
);
INSERT INTO speaker (id, interview_id, label, color, display_name)
VALUES (1, 1, 'Intervenant 1', '#3156a3', NULL);
INSERT INTO segment (
    id, interview_id, speaker_id, start_ms, end_ms, raw_text, confidence, status
) VALUES (
    1, 1, 1, 1000, 2500, 'Texte synthetique immuable.', 0.9, 'raw'
);
INSERT INTO edit (id, segment_id, operation, before_text, after_text, created_at)
VALUES (
    1, 1, 'manual', 'Texte synthetique immuable.',
    'Texte synthetique edite.', '1700000001'
);
INSERT INTO setting (
    interview_id, show_timestamps, model_path, language, cleanup_enabled, device
) VALUES (1, 0, '/fixture/model.bin', 'fr', 1, 'cpu');
