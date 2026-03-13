-- Seed with well-known gambling domains
-- Source: public gambling domain blocklists
INSERT INTO blocklist_entries (domain, category, source, confidence, status, tags, created_at, updated_at)
VALUES
    ('bet365.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator', 'uk'], NOW(), NOW()),
    ('draftkings.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator', 'us'], NOW(), NOW()),
    ('fanduel.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator', 'us'], NOW(), NOW()),
    ('pokerstars.com', 'poker', 'curated', 1.0, 'active', ARRAY['major_operator'], NOW(), NOW()),
    ('888casino.com', 'online_casino', 'curated', 1.0, 'active', ARRAY['major_operator'], NOW(), NOW()),
    ('betway.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator'], NOW(), NOW()),
    ('williamhill.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator', 'uk'], NOW(), NOW()),
    ('bovada.lv', 'online_casino', 'curated', 1.0, 'active', ARRAY['major_operator'], NOW(), NOW()),
    ('betmgm.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator', 'us'], NOW(), NOW()),
    ('caesars.com', 'online_casino', 'curated', 1.0, 'active', ARRAY['major_operator', 'us'], NOW(), NOW()),
    ('paddypower.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator', 'uk'], NOW(), NOW()),
    ('ladbrokes.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator', 'uk'], NOW(), NOW()),
    ('unibet.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator'], NOW(), NOW()),
    ('stake.com', 'crypto_gambling', 'curated', 1.0, 'active', ARRAY['crypto', 'major_operator'], NOW(), NOW()),
    ('roobet.com', 'crypto_gambling', 'curated', 1.0, 'active', ARRAY['crypto'], NOW(), NOW());

-- Create initial blocklist version
INSERT INTO blocklist_versions (version_number, entry_count, signature, published_at)
VALUES (1, 15, E'\\x00', NOW());
