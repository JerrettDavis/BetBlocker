-- Seed blocklist with initial well-known gambling domains

INSERT INTO blocklist_entries (domain, category, source, confidence, status)
VALUES
    ('bet365.com', 'sports_betting', 'curated', 100, 'active'),
    ('pokerstars.com', 'poker', 'curated', 100, 'active'),
    ('888casino.com', 'casino', 'curated', 100, 'active'),
    ('williamhill.com', 'sports_betting', 'curated', 100, 'active'),
    ('paddypower.com', 'sports_betting', 'curated', 100, 'active'),
    ('betfair.com', 'sports_betting', 'curated', 100, 'active'),
    ('ladbrokes.com', 'sports_betting', 'curated', 100, 'active'),
    ('coral.co.uk', 'sports_betting', 'curated', 100, 'active'),
    ('draftkings.com', 'sports_betting', 'curated', 100, 'active'),
    ('fanduel.com', 'sports_betting', 'curated', 100, 'active'),
    ('betway.com', 'sports_betting', 'curated', 100, 'active'),
    ('unibet.com', 'sports_betting', 'curated', 100, 'active'),
    ('bwin.com', 'sports_betting', 'curated', 100, 'active'),
    ('partypoker.com', 'poker', 'curated', 100, 'active'),
    ('888poker.com', 'poker', 'curated', 100, 'active'),
    ('casumo.com', 'casino', 'curated', 100, 'active'),
    ('leovegas.com', 'casino', 'curated', 100, 'active'),
    ('mrgreen.com', 'casino', 'curated', 100, 'active'),
    ('betsson.com', 'sports_betting', 'curated', 100, 'active'),
    ('bovada.lv', 'casino', 'curated', 100, 'active')
ON CONFLICT (domain) DO NOTHING;

-- Create initial blocklist version
INSERT INTO blocklist_versions (version_number, entry_count, delta_metadata)
VALUES (1, 20, '{"initial_seed": true}');
