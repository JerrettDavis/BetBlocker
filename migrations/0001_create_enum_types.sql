-- Enum type definitions for BetBlocker

-- Required extensions
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- Account role within the platform
CREATE TYPE account_role AS ENUM (
    'user',
    'partner',
    'authority',
    'admin'
);

-- Device operating system / platform
CREATE TYPE device_platform AS ENUM (
    'windows',
    'macos',
    'linux',
    'android',
    'ios'
);

-- Device lifecycle status
CREATE TYPE device_status AS ENUM (
    'pending',
    'active',
    'stale',
    'disabled',
    'unenrolling',
    'unenrolled',
    'decommissioned'
);

-- Enrollment tier
CREATE TYPE enrollment_tier AS ENUM (
    'self',
    'partner',
    'authority'
);

-- Enrollment lifecycle status
CREATE TYPE enrollment_status AS ENUM (
    'active',
    'suspended',
    'unenrolling',
    'unenrolled',
    'unenroll_requested',
    'unenroll_approved'
);

-- Unenrollment request status
CREATE TYPE unenroll_request_status AS ENUM (
    'pending',
    'approved',
    'denied',
    'expired',
    'cancelled'
);

-- Partner relationship status
CREATE TYPE partner_status AS ENUM (
    'invited',
    'active',
    'revoked',
    'expired'
);

-- Organization type
CREATE TYPE org_type AS ENUM (
    'family',
    'therapy',
    'court',
    'enterprise'
);

-- Organization member role
CREATE TYPE org_member_role AS ENUM (
    'admin',
    'member'
);

-- Blocklist entry source
CREATE TYPE blocklist_source AS ENUM (
    'curated',
    'automated',
    'federated'
);

-- Blocklist entry status
CREATE TYPE blocklist_entry_status AS ENUM (
    'active',
    'review',
    'rejected'
);

-- Federated report review status
CREATE TYPE federated_report_status AS ENUM (
    'pending',
    'confirmed',
    'rejected',
    'duplicate',
    'promoted'
);

-- Event type emitted by device agents
CREATE TYPE event_type AS ENUM (
    'block',
    'bypass_attempt',
    'tamper',
    'heartbeat',
    'config_change',
    'unenroll',
    'app_block',
    'install_block',
    'vpn_detected',
    'extension_removed'
);

-- Subscription plan (hosted only)
CREATE TYPE subscription_plan AS ENUM (
    'standard',
    'partner',
    'institutional'
);

-- Subscription lifecycle status
CREATE TYPE subscription_status AS ENUM (
    'trialing',
    'active',
    'past_due',
    'canceled',
    'unpaid',
    'paused'
);

-- Audit log action
CREATE TYPE audit_action AS ENUM (
    'INSERT',
    'UPDATE',
    'DELETE'
);
