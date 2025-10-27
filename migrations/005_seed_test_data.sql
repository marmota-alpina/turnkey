-- Migration: Seed test data for development and testing
-- Creates sample users and cards for MVP testing
-- This data should NOT be used in production

-- Test users (10 users with different configurations)
INSERT INTO users (pis, nome, matricula, cpf, validade_inicio, validade_fim, ativo, allow_card, allow_bio, allow_keypad, codigo) VALUES
    -- Active users with different access methods
    ('12345678901', 'João da Silva', '1001', '12345678901', '2025-01-01T00:00:00Z', '2025-12-31T23:59:59Z', 1, 1, 1, 1, '1234'),
    ('23456789012', 'Maria Santos', '1002', '23456789012', '2025-01-01T00:00:00Z', NULL, 1, 1, 0, 1, '5678'),
    ('34567890123', 'Pedro Oliveira', '1003', '34567890123', '2025-01-01T00:00:00Z', '2025-12-31T23:59:59Z', 1, 1, 1, 0, NULL),
    ('45678901234', 'Ana Costa', '1004', '45678901234', '2025-01-01T00:00:00Z', '2025-06-30T23:59:59Z', 1, 1, 0, 0, NULL),
    (NULL, 'Carlos Mendes', '1005', NULL, NULL, NULL, 1, 1, 1, 1, '9999'),

    -- Inactive user (for testing denial)
    ('56789012345', 'Usuário Inativo', '1006', '56789012345', '2025-01-01T00:00:00Z', '2025-12-31T23:59:59Z', 0, 1, 0, 1, '0000'),

    -- Expired user (for testing date range)
    ('67890123456', 'Usuário Expirado', '1007', '67890123456', '2024-01-01T00:00:00Z', '2024-12-31T23:59:59Z', 1, 1, 0, 0, NULL),

    -- Future user (not yet valid)
    ('78901234567', 'Usuário Futuro', '1008', '78901234567', '2026-01-01T00:00:00Z', '2026-12-31T23:59:59Z', 1, 1, 0, 0, NULL),

    -- Card-only user
    ('89012345678', 'Apenas Cartão', '1009', '89012345678', NULL, NULL, 1, 1, 0, 0, NULL),

    -- Biometric-only user
    ('90123456789', 'Apenas Biometria', '1010', '90123456789', NULL, NULL, 1, 0, 1, 0, NULL);

-- Test cards (15 cards, including multiple cards per user)
INSERT INTO cards (numero_cartao, matricula, user_id, validade_inicio, validade_fim, ativo) VALUES
    -- Primary cards for users 1001-1005
    ('00000000000011912322', '1001', 1, '2025-01-01T00:00:00Z', '2025-12-31T23:59:59Z', 1),
    ('00000000000022823433', '1002', 2, '2025-01-01T00:00:00Z', NULL, 1),
    ('00000000000033734544', '1003', 3, '2025-01-01T00:00:00Z', '2025-12-31T23:59:59Z', 1),
    ('00000000000044645655', '1004', 4, '2025-01-01T00:00:00Z', '2025-06-30T23:59:59Z', 1),
    ('00000000000055556766', '1005', 5, NULL, NULL, 1),

    -- Secondary/backup cards for user 1001
    ('00000000000099988877', '1001', 1, '2025-01-01T00:00:00Z', '2025-12-31T23:59:59Z', 1),

    -- Cards for special test cases
    ('00000000000066667777', '1006', 6, '2025-01-01T00:00:00Z', '2025-12-31T23:59:59Z', 1),  -- Inactive user
    ('00000000000077778888', '1007', 7, '2024-01-01T00:00:00Z', '2024-12-31T23:59:59Z', 1),  -- Expired user
    ('00000000000088889999', '1008', 8, '2026-01-01T00:00:00Z', '2026-12-31T23:59:59Z', 1),  -- Future user
    ('00000000000099990000', '1009', 9, NULL, NULL, 1),                                       -- Card-only user

    -- Inactive card (for testing)
    ('00000000000011112222', '1001', 1, '2025-01-01T00:00:00Z', '2025-12-31T23:59:59Z', 0),

    -- Hexadecimal format cards (MIFARE)
    ('ABCDEF123456', '1002', 2, '2025-01-01T00:00:00Z', NULL, 1),
    ('123456ABCDEF', '1003', 3, '2025-01-01T00:00:00Z', '2025-12-31T23:59:59Z', 1),

    -- Short format cards
    ('12345', '1004', 4, '2025-01-01T00:00:00Z', '2025-06-30T23:59:59Z', 1),
    ('ABC123', '1005', 5, NULL, NULL, 1);

-- Note: Biometric templates are NOT seeded here because they contain binary data
-- For testing biometric functionality, use the import feature with biometria.txt
-- or create templates programmatically in tests

-- Sample access logs for testing reports
INSERT INTO access_logs (user_id, matricula, card_number, direction, reader_type, granted, display_message, timestamp) VALUES
    -- Successful accesses
    (1, '1001', '00000000000011912322', 1, 1, 1, 'Acesso liberado', '2025-10-26T08:00:00Z'),
    (2, '1002', '00000000000022823433', 1, 1, 1, 'Acesso liberado', '2025-10-26T08:15:00Z'),
    (3, '1003', '00000000000033734544', 2, 1, 1, 'Acesso liberado', '2025-10-26T12:00:00Z'),
    (1, '1001', '00000000000011912322', 2, 1, 1, 'Acesso liberado', '2025-10-26T18:00:00Z'),

    -- Denied accesses (for security monitoring)
    (6, '1006', '00000000000066667777', 1, 1, 0, 'Usuário inativo', '2025-10-26T09:00:00Z'),
    (7, '1007', '00000000000077778888', 1, 1, 0, 'Fora do período de validade', '2025-10-26T09:30:00Z'),
    (NULL, NULL, '99999999999999999999', 1, 1, 0, 'Cartão não cadastrado', '2025-10-26T10:00:00Z'),
    (1, '1001', '00000000000011112222', 1, 1, 0, 'Cartão inativo', '2025-10-26T10:30:00Z');

-- Verify seed data
SELECT 'Seed completed: ' ||
       (SELECT COUNT(*) FROM users) || ' users, ' ||
       (SELECT COUNT(*) FROM cards) || ' cards, ' ||
       (SELECT COUNT(*) FROM access_logs) || ' access logs' as result;
