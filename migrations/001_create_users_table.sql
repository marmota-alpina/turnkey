-- Migration: Create users table
-- Compatible with colaborador.txt format from real Henry equipment
-- Format: PIS|NOME|MATRICULA|CPF|VALIDADE_INICIO|VALIDADE_FIM|ATIVO|ALLOW_CARD|ALLOW_BIO|ALLOW_KEYPAD|CODIGO

CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Identification (Brazilian standard format)
    pis TEXT,                           -- PIS: 11 digits (optional)
    nome TEXT NOT NULL,                 -- Full name (max 100 chars)
    matricula TEXT NOT NULL UNIQUE,     -- Unique employee ID (3-20 chars)
    cpf TEXT,                           -- CPF: 11 digits (optional)

    -- Validity period (ISO8601 format)
    validade_inicio TEXT,               -- ISO8601: '2025-01-01T00:00:00Z'
    validade_fim TEXT,                  -- ISO8601: '2025-12-31T23:59:59Z'

    -- Status
    ativo BOOLEAN NOT NULL DEFAULT 1,   -- 1=active, 0=inactive

    -- Allowed access methods
    allow_card BOOLEAN NOT NULL DEFAULT 1,      -- Allow RFID card
    allow_bio BOOLEAN NOT NULL DEFAULT 0,       -- Allow biometric
    allow_keypad BOOLEAN NOT NULL DEFAULT 0,    -- Allow keypad password

    -- Numeric password (for keypad)
    codigo TEXT,                        -- Numeric code (max 20 chars)

    -- Metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Constraints
    CHECK (LENGTH(pis) = 11 OR pis IS NULL),
    CHECK (LENGTH(cpf) = 11 OR cpf IS NULL),
    CHECK (LENGTH(matricula) >= 3 AND LENGTH(matricula) <= 20),
    CHECK (LENGTH(nome) <= 100),
    CHECK (allow_card = 1 OR allow_bio = 1 OR allow_keypad = 1),  -- At least one method required
    CHECK (allow_keypad = 0 OR codigo IS NOT NULL)                 -- Keypad requires code
);

-- Indices for performance
CREATE INDEX idx_users_matricula ON users(matricula);
CREATE INDEX idx_users_ativo ON users(ativo);
CREATE INDEX idx_users_cpf ON users(cpf) WHERE cpf IS NOT NULL;
CREATE INDEX idx_users_pis ON users(pis) WHERE pis IS NOT NULL;
CREATE INDEX idx_users_nome ON users(nome);
CREATE INDEX idx_users_allow_card ON users(allow_card) WHERE allow_card = 1;
CREATE INDEX idx_users_allow_bio ON users(allow_bio) WHERE allow_bio = 1;
CREATE INDEX idx_users_allow_keypad ON users(allow_keypad) WHERE allow_keypad = 1;

-- Trigger to update updated_at timestamp
CREATE TRIGGER update_users_timestamp
AFTER UPDATE ON users
FOR EACH ROW
BEGIN
    UPDATE users SET updated_at = datetime('now') WHERE id = NEW.id;
END;
