-- Migration: Create biometric_templates table
-- Compatible with biometria.txt format from real Henry equipment
-- Format: MATRICULA|POSICAO|TEMPLATE_BASE64
-- Stores fingerprint templates (Base64 encoded in import file, binary in DB)

CREATE TABLE IF NOT EXISTS biometric_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- User relationship (dual-key for compatibility + performance)
    matricula TEXT NOT NULL,            -- FK to users.matricula
    user_id INTEGER NOT NULL,           -- FK to users.id

    -- Finger position (0-9)
    posicao INTEGER NOT NULL,           -- 0=R.Thumb, 1=R.Index, ..., 9=L.Pinky

    -- Biometric template (BLOB)
    template_data BLOB NOT NULL,        -- Binary template data (500-2000 bytes)

    -- Metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Constraints
    CHECK (posicao >= 0 AND posicao <= 9),
    CHECK (LENGTH(template_data) >= 500 AND LENGTH(template_data) <= 2000),
    FOREIGN KEY (matricula) REFERENCES users(matricula) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(matricula, posicao)  -- No duplicate finger positions per user
);

-- Indices for performance
CREATE INDEX idx_biometric_matricula ON biometric_templates(matricula);
CREATE INDEX idx_biometric_user_id ON biometric_templates(user_id);
CREATE INDEX idx_biometric_posicao ON biometric_templates(posicao);
CREATE INDEX idx_biometric_user_position ON biometric_templates(user_id, posicao);

-- Trigger to update updated_at timestamp
CREATE TRIGGER update_biometric_templates_timestamp
AFTER UPDATE ON biometric_templates
FOR EACH ROW
BEGIN
    UPDATE biometric_templates SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- Trigger to enforce dual-key consistency
CREATE TRIGGER enforce_biometric_user_consistency_insert
BEFORE INSERT ON biometric_templates
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'matricula and user_id must reference same user')
    WHERE NEW.user_id != (SELECT id FROM users WHERE matricula = NEW.matricula);
END;

CREATE TRIGGER enforce_biometric_user_consistency_update
BEFORE UPDATE OF matricula, user_id ON biometric_templates
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'matricula and user_id must reference same user')
    WHERE NEW.user_id != (SELECT id FROM users WHERE matricula = NEW.matricula);
END;

-- Trigger to verify user has ALLOW_BIO enabled
CREATE TRIGGER enforce_biometric_user_permission
BEFORE INSERT ON biometric_templates
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'User does not have ALLOW_BIO enabled')
    WHERE (SELECT allow_bio FROM users WHERE id = NEW.user_id) = 0;
END;
