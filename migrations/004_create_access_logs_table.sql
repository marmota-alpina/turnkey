-- Migration: Create access_logs table
-- Logs all access attempts (granted and denied)
-- Used for audit trail, reports, and security monitoring

CREATE TABLE IF NOT EXISTS access_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Identification (NULL if card/user not found)
    user_id INTEGER,                    -- FK to users.id (NULL if card not registered)
    matricula TEXT,                     -- FK to users.matricula (NULL if not found)
    card_number TEXT NOT NULL,          -- Card number used (always filled, even if invalid)

    -- Access details
    direction INTEGER NOT NULL,         -- 0=Undefined, 1=Entry, 2=Exit
    reader_type INTEGER NOT NULL,       -- 1=RFID, 5=Biometric
    granted BOOLEAN NOT NULL,           -- true=granted, false=denied

    -- Display message shown to user
    display_message TEXT,               -- Message shown on LCD (e.g., "Acesso liberado", "Cartão inativo")

    -- Timestamps (ISO8601 format)
    timestamp TEXT NOT NULL,            -- ISO8601: when access occurred
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Constraints
    CHECK (direction >= 0 AND direction <= 2),
    CHECK (reader_type IN (1, 5)),
    CHECK (LENGTH(card_number) >= 3 AND LENGTH(card_number) <= 20),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL,
    FOREIGN KEY (matricula) REFERENCES users(matricula) ON DELETE SET NULL
);

-- Indices for performance and reporting
CREATE INDEX idx_access_logs_timestamp ON access_logs(timestamp DESC);
CREATE INDEX idx_access_logs_user_id ON access_logs(user_id);
CREATE INDEX idx_access_logs_matricula ON access_logs(matricula);
CREATE INDEX idx_access_logs_card_number ON access_logs(card_number);
CREATE INDEX idx_access_logs_granted ON access_logs(granted);
CREATE INDEX idx_access_logs_direction ON access_logs(direction);
CREATE INDEX idx_access_logs_reader_type ON access_logs(reader_type);

-- Composite indices for common queries
CREATE INDEX idx_access_logs_user_timestamp ON access_logs(user_id, timestamp DESC);
CREATE INDEX idx_access_logs_granted_timestamp ON access_logs(granted, timestamp DESC);
CREATE INDEX idx_access_logs_card_timestamp ON access_logs(card_number, timestamp DESC);

-- View for recent denied accesses (security monitoring)
CREATE VIEW IF NOT EXISTS recent_denied_accesses AS
SELECT
    al.id,
    al.card_number,
    al.matricula,
    u.nome as user_name,
    al.direction,
    al.reader_type,
    al.display_message,
    al.timestamp
FROM access_logs al
LEFT JOIN users u ON al.user_id = u.id
WHERE al.granted = 0
ORDER BY al.timestamp DESC
LIMIT 100;

-- View for daily access statistics
CREATE VIEW IF NOT EXISTS daily_access_stats AS
SELECT
    DATE(timestamp) as date,
    COUNT(*) as total_attempts,
    SUM(CASE WHEN granted = 1 THEN 1 ELSE 0 END) as granted_count,
    SUM(CASE WHEN granted = 0 THEN 1 ELSE 0 END) as denied_count,
    SUM(CASE WHEN direction = 1 THEN 1 ELSE 0 END) as entry_count,
    SUM(CASE WHEN direction = 2 THEN 1 ELSE 0 END) as exit_count
FROM access_logs
GROUP BY DATE(timestamp)
ORDER BY date DESC;
