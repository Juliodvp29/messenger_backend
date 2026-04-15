-- 018_performance_autovacuum.sql
-- ============================================================
-- Performance: Configure autovacuum for high-write tables

-- Messages table: highest write volume
ALTER TABLE messages SET (
    autovacuum_vacuum_scale_factor = 0.01,
    autovacuum_analyze_scale_factor = 0.005,
    autovacuum_vacuum_threshold = 1000,
    autovacuum_analyze_threshold = 500
);

-- message_status table: high writes on status updates
ALTER TABLE message_status SET (
    autovacuum_vacuum_scale_factor = 0.01,
    autovacuum_analyze_scale_factor = 0.005,
    autovacuum_vacuum_threshold = 1000,
    autovacuum_analyze_threshold = 500
);

-- message_reactions: moderate writes
ALTER TABLE message_reactions SET (
    autovacuum_vacuum_scale_factor = 0.02,
    autovacuum_analyze_scale_factor = 0.01
);

-- user_sessions: moderate writes
ALTER TABLE user_sessions SET (
    autovacuum_vacuum_scale_factor = 0.02,
    autovacuum_analyze_scale_factor = 0.01
);

-- stories: moderate writes, batch deletes
ALTER TABLE stories SET (
    autovacuum_vacuum_scale_factor = 0.02,
    autovacuum_analyze_scale_factor = 0.01
);

-- ANALYZE to update statistics immediately
ANALYZE messages;
ANALYZE message_status;
ANALYZE message_reactions;
ANALYZE user_sessions;
ANALYZE stories;
