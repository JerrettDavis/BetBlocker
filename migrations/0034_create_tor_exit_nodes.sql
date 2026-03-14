-- Migration 0034: create tor_exit_nodes table
--
-- Stores Tor exit node IP addresses fetched by the bb-worker refresh job.
-- Using the PostgreSQL `inet` type for proper IP handling and indexing.

CREATE TABLE IF NOT EXISTS tor_exit_nodes (
    ip_address  inet        NOT NULL,
    fetched_at  timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT tor_exit_nodes_pkey PRIMARY KEY (ip_address)
);

COMMENT ON TABLE tor_exit_nodes IS
    'Current list of Tor exit node IPs, refreshed every 6 hours by bb-worker.';
