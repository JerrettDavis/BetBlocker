-- Seed app signatures for top gambling applications

INSERT INTO app_signatures (name, package_names, executable_names, display_name_patterns, platforms, category, status, confidence, source)
VALUES
(
    'Bet365',
    ARRAY['com.bet365.app', 'com.bet365.casino', 'com.bet365.poker'],
    ARRAY['bet365.exe', 'Bet365.app'],
    ARRAY['Bet365*', '*bet365*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'sports_betting',
    'active',
    1.0,
    'curated'
),
(
    'PokerStars',
    ARRAY['com.pokerstars.app', 'com.pokerstars.casino', 'com.pokerstars.sports'],
    ARRAY['PokerStars.exe', 'PokerStars.app', 'PokerStarsUpdate.exe'],
    ARRAY['PokerStars*', '*pokerstars*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'poker',
    'active',
    1.0,
    'curated'
),
(
    'DraftKings',
    ARRAY['com.draftkings.sportsbook', 'com.draftkings.casino', 'com.draftkings.dfs'],
    ARRAY['DraftKings.exe', 'DraftKings.app'],
    ARRAY['DraftKings*', '*draftkings*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'sports_betting',
    'active',
    1.0,
    'curated'
),
(
    'FanDuel',
    ARRAY['com.fanduel.sportsbook', 'com.fanduel.casino', 'com.fanduel.dfs'],
    ARRAY['FanDuel.exe', 'FanDuel.app'],
    ARRAY['FanDuel*', '*fanduel*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'sports_betting',
    'active',
    1.0,
    'curated'
),
(
    'BetMGM',
    ARRAY['com.betmgm.sportsbook', 'com.betmgm.casino', 'com.betmgm.poker'],
    ARRAY['BetMGM.exe', 'BetMGM.app'],
    ARRAY['BetMGM*', '*betmgm*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'sports_betting',
    'active',
    1.0,
    'curated'
),
(
    'William Hill',
    ARRAY['com.williamhill.sportsbook', 'com.williamhill.casino', 'com.williamhill.poker'],
    ARRAY['WilliamHill.exe', 'William Hill.app'],
    ARRAY['William Hill*', '*williamhill*', '*William Hill*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'sports_betting',
    'active',
    1.0,
    'curated'
),
(
    'Paddy Power',
    ARRAY['com.paddypower.sportsbook', 'com.paddypower.casino', 'com.paddypower.games'],
    ARRAY['PaddyPower.exe', 'Paddy Power.app'],
    ARRAY['Paddy Power*', '*paddypower*', '*Paddy Power*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'sports_betting',
    'active',
    1.0,
    'curated'
),
(
    '888poker',
    ARRAY['com.poker888.app', 'com.casino888.app'],
    ARRAY['888poker.exe', '888poker.app', '888casino.exe'],
    ARRAY['888poker*', '888casino*', '*888poker*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'poker',
    'active',
    1.0,
    'curated'
),
(
    'PartyPoker',
    ARRAY['com.partypoker.app', 'com.partycasino.app'],
    ARRAY['PartyPoker.exe', 'PartyPoker.app'],
    ARRAY['PartyPoker*', '*partypoker*', '*Party Poker*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'poker',
    'active',
    1.0,
    'curated'
),
(
    'Unibet',
    ARRAY['com.unibet.sportsbook', 'com.unibet.casino', 'com.unibet.poker'],
    ARRAY['Unibet.exe', 'Unibet.app'],
    ARRAY['Unibet*', '*unibet*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'sports_betting',
    'active',
    1.0,
    'curated'
),
(
    'Betfair',
    ARRAY['com.betfair.sportsbook', 'com.betfair.exchange', 'com.betfair.casino'],
    ARRAY['Betfair.exe', 'Betfair.app'],
    ARRAY['Betfair*', '*betfair*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'sports_betting',
    'active',
    1.0,
    'curated'
),
(
    'Ladbrokes',
    ARRAY['com.ladbrokes.sportsbook', 'com.ladbrokes.casino', 'com.ladbrokes.games'],
    ARRAY['Ladbrokes.exe', 'Ladbrokes.app'],
    ARRAY['Ladbrokes*', '*ladbrokes*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'sports_betting',
    'active',
    1.0,
    'curated'
),
(
    'Coral',
    ARRAY['com.coral.sportsbook', 'com.coral.casino', 'com.coral.games'],
    ARRAY['Coral.exe', 'Coral.app'],
    ARRAY['Coral*', '*coral.co.uk*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'sports_betting',
    'active',
    1.0,
    'curated'
),
(
    'Sky Bet',
    ARRAY['com.skybet.sportsbook', 'com.skybet.casino', 'com.skybet.vegas'],
    ARRAY['SkyBet.exe', 'Sky Bet.app'],
    ARRAY['Sky Bet*', 'SkyBet*', '*skybet*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'sports_betting',
    'active',
    1.0,
    'curated'
),
(
    'bwin',
    ARRAY['com.bwin.sportsbook', 'com.bwin.casino', 'com.bwin.poker'],
    ARRAY['bwin.exe', 'bwin.app'],
    ARRAY['bwin*', '*bwin*'],
    ARRAY['windows', 'macos', 'android', 'ios'],
    'sports_betting',
    'active',
    1.0,
    'curated'
);
