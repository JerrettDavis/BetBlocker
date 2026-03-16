-- Seed blocklist with curated gambling domains.
--
-- Domains under PAYMENT_PROCESSORS are inserted with status = 'review'
-- so they are NOT active by default.

INSERT INTO blocklist_entries (domain, category, source, confidence, status)
VALUES
    -- SPORTS_BETTING
    ('bet365.com', 'sports_betting', 'curated', 100, 'active'),
    ('williamhill.com', 'sports_betting', 'curated', 100, 'active'),
    ('ladbrokes.com', 'sports_betting', 'curated', 100, 'active'),
    ('coral.co.uk', 'sports_betting', 'curated', 100, 'active'),
    ('paddypower.com', 'sports_betting', 'curated', 100, 'active'),
    ('betfair.com', 'sports_betting', 'curated', 100, 'active'),
    ('betway.com', 'sports_betting', 'curated', 100, 'active'),
    ('888sport.com', 'sports_betting', 'curated', 100, 'active'),
    ('unibet.com', 'sports_betting', 'curated', 100, 'active'),
    ('bwin.com', 'sports_betting', 'curated', 100, 'active'),
    ('betfred.com', 'sports_betting', 'curated', 100, 'active'),
    ('skybet.com', 'sports_betting', 'curated', 100, 'active'),
    ('draftkings.com', 'sports_betting', 'curated', 100, 'active'),
    ('fanduel.com', 'sports_betting', 'curated', 100, 'active'),
    ('pointsbet.com', 'sports_betting', 'curated', 100, 'active'),
    ('betmgm.com', 'sports_betting', 'curated', 100, 'active'),
    ('betrivers.com', 'sports_betting', 'curated', 100, 'active'),
    ('hardrock.bet', 'sports_betting', 'curated', 100, 'active'),

    -- ONLINE_CASINOS
    ('888casino.com', 'online_casinos', 'curated', 100, 'active'),
    ('partycasino.com', 'online_casinos', 'curated', 100, 'active'),
    ('leovegas.com', 'online_casinos', 'curated', 100, 'active'),
    ('casumo.com', 'online_casinos', 'curated', 100, 'active'),
    ('mrgreen.com', 'online_casinos', 'curated', 100, 'active'),
    ('betsson.com', 'online_casinos', 'curated', 100, 'active'),
    ('videoslots.com', 'online_casinos', 'curated', 100, 'active'),
    ('jackpotcity.com', 'online_casinos', 'curated', 100, 'active'),
    ('playojo.com', 'online_casinos', 'curated', 100, 'active'),
    ('wildz.com', 'online_casinos', 'curated', 100, 'active'),

    -- POKER
    ('pokerstars.com', 'poker', 'curated', 100, 'active'),
    ('888poker.com', 'poker', 'curated', 100, 'active'),
    ('partypoker.com', 'poker', 'curated', 100, 'active'),
    ('ggpoker.com', 'poker', 'curated', 100, 'active'),
    ('wsop.com', 'poker', 'curated', 100, 'active'),
    ('globalpoker.com', 'poker', 'curated', 100, 'active'),

    -- LOTTERY
    ('lottoland.com', 'lottery', 'curated', 100, 'active'),
    ('jackpot.com', 'lottery', 'curated', 100, 'active'),
    ('thelotter.com', 'lottery', 'curated', 100, 'active'),

    -- BINGO
    ('tombola.co.uk', 'bingo', 'curated', 100, 'active'),
    ('meccabingo.com', 'bingo', 'curated', 100, 'active'),
    ('winkbingo.com', 'bingo', 'curated', 100, 'active'),

    -- FANTASY_SPORTS
    ('underdogfantasy.com', 'fantasy_sports', 'curated', 100, 'active'),
    ('prizepicks.com', 'fantasy_sports', 'curated', 100, 'active'),

    -- SPREAD_BETTING
    ('spreadex.com', 'spread_betting', 'curated', 100, 'active'),

    -- PAYMENT_PROCESSORS (review status — not active by default)
    ('skrill.com', 'payment_processors', 'curated', 100, 'review'),
    ('neteller.com', 'payment_processors', 'curated', 100, 'review'),
    ('paysafecard.com', 'payment_processors', 'curated', 100, 'review'),
    ('muchbetter.com', 'payment_processors', 'curated', 100, 'review')
ON CONFLICT (domain) DO NOTHING;

-- Create initial blocklist version with accurate count
INSERT INTO blocklist_versions (version_number, entry_count, delta_metadata)
SELECT
    1,
    count(*),
    jsonb_build_object(
        'initial_seed', true,
        'seeded_at',    now()::text
    )
FROM blocklist_entries
WHERE source = 'curated'
  AND status = 'active';
