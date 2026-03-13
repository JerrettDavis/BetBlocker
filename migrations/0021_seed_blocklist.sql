-- Seed blocklist from data/blocklist-seed.txt
--
-- The seed file uses category headers (lines like "# CATEGORY_NAME") to group
-- domains.  This migration reads the file, parses category context, and inserts
-- each domain with the appropriate category.
--
-- Domains under "# PAYMENT_PROCESSORS (optional, disabled by default)" are
-- inserted with status = 'review' so they are NOT active by default.

DO $$
DECLARE
    line           TEXT;
    current_cat    VARCHAR(100) := 'uncategorized';
    is_payment     BOOLEAN := FALSE;
    domain_count   INT := 0;
    raw            TEXT;
    lines          TEXT[];
BEGIN
    -- Read the seed file (path relative to the data directory shipped alongside migrations)
    raw := pg_read_file('data/blocklist-seed.txt');
    lines := string_to_array(raw, E'\n');

    FOREACH line IN ARRAY lines
    LOOP
        -- Trim whitespace
        line := btrim(line);

        -- Skip empty lines
        IF line = '' THEN
            CONTINUE;
        END IF;

        -- Detect category headers
        IF line ~ '^#\s+[A-Z_]+' THEN
            -- Extract category name (first word after #)
            current_cat := lower(btrim((regexp_match(line, '^#\s+([A-Z_]+)'))[1]));

            IF current_cat = 'payment_processors' THEN
                is_payment := TRUE;
            END IF;

            CONTINUE;
        END IF;

        -- Skip other comment lines (descriptions, sub-comments)
        IF line ~ '^#' THEN
            CONTINUE;
        END IF;

        -- Insert the domain
        INSERT INTO blocklist_entries (domain, category, source, confidence, status)
        VALUES (
            line,
            current_cat,
            'curated',
            100,
            CASE WHEN is_payment THEN 'review' ELSE 'active' END
        )
        ON CONFLICT (domain) DO NOTHING;

        domain_count := domain_count + 1;
    END LOOP;

    RAISE NOTICE 'Seeded % domains from blocklist-seed.txt', domain_count;
END;
$$;

-- Create initial blocklist version with accurate count
INSERT INTO blocklist_versions (version_number, entry_count, delta_metadata)
SELECT
    1,
    count(*),
    jsonb_build_object(
        'initial_seed', true,
        'source_file',  'data/blocklist-seed.txt',
        'seeded_at',    now()::text
    )
FROM blocklist_entries
WHERE source = 'curated'
  AND status = 'active';
