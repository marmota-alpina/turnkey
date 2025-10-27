-- Migration: Create cards table
-- Compatible with cartoes.txt format from real Henry equipment
-- Format: NUMERO_CARTAO|MATRICULA|VALIDADE_INICIO|VALIDADE_FIM|ATIVO
-- Supports multiple cards per user (primary, backup, vehicle access, etc.)

CREATE TABLE IF NOT EXISTS cards (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Card number (decimal or hexadecimal format)
    numero_cartao TEXT NOT NULL UNIQUE, -- 3-20 chars (e.g., "00000000000011912322" or "ABCDEF123456")

    -- User relationship (dual-key for compatibility + performance)
    matricula TEXT NOT NULL,            -- FK to users.matricula
    user_id INTEGER NOT NULL,           -- FK to users.id (denormalized for performance)

    -- Validity period (ISO8601 format)
    validade_inicio TEXT,               -- ISO8601
    validade_fim TEXT,                  -- ISO8601

    -- Status
    ativo BOOLEAN NOT NULL DEFAULT 1,   -- 1=active, 0=inactive

    -- Metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Constraints
    CHECK (LENGTH(numero_cartao) >= 3 AND LENGTH(numero_cartao) <= 20),
    FOREIGN KEY (matricula) REFERENCES users(matricula) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Indices for performance
CREATE UNIQUE INDEX idx_cards_numero_cartao ON cards(numero_cartao);
CREATE INDEX idx_cards_matricula ON cards(matricula);
CREATE INDEX idx_cards_user_id ON cards(user_id);
CREATE INDEX idx_cards_ativo ON cards(ativo);
CREATE INDEX idx_cards_user_active ON cards(user_id, ativo) WHERE ativo = 1;

-- Trigger to update updated_at timestamp
CREATE TRIGGER update_cards_timestamp
AFTER UPDATE ON cards
FOR EACH ROW
BEGIN
    UPDATE cards SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- Trigger to enforce dual-key consistency
CREATE TRIGGER enforce_card_user_consistency_insert
BEFORE INSERT ON cards
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'matricula and user_id must reference same user')
    WHERE NEW.user_id != (SELECT id FROM users WHERE matricula = NEW.matricula);
END;

CREATE TRIGGER enforce_card_user_consistency_update
BEFORE UPDATE OF matricula, user_id ON cards
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'matricula and user_id must reference same user')
    WHERE NEW.user_id != (SELECT id FROM users WHERE matricula = NEW.matricula);
END;
