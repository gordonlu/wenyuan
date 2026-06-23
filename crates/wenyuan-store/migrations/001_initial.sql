create table if not exists sessions (
    id text primary key,
    title text not null,
    topic text not null,
    context text not null,
    mode text not null default 'three_seat',
    phase text not null,
    created_at text not null,
    updated_at text not null,
    result_json text,
    failure_reason text,
    convergence_used integer not null default 0,
    artifacts_json text,
    execution_token text,
    lease_expires_at text,
    recovery_state text not null default 'idle',
    model_config text,
    vote_policy text,
    scribe_enabled integer not null default 0,
    search_enabled integer not null default 0,
    external_evidence_json text not null default '[]',
    external_tool_runs_json text not null default '[]'
);

create table if not exists seats (
    session_id text not null,
    seat_kind text not null,
    status text not null,
    last_error text,
    system_prompt text not null default '',
    conversation_json text not null default '[]',
    provider_ref text not null default 'default',
    primary key (session_id, seat_kind)
);

create table if not exists seat_runs (
    id text primary key,
    session_id text not null,
    seat_kind text not null,
    phase text not null,
    status text not null,
    raw_output text,
    error text,
    prompt_version text,
    repair_attempted integer not null default 0,
    duration_ms text,
    prompt_tokens integer,
    completion_tokens integer,
    total_tokens integer,
    upstream_status integer
);

create table if not exists rounds (
    id text primary key,
    session_id text not null,
    phase text not null,
    round_index integer not null
);

create table if not exists ideas (
    id text primary key,
    session_id text not null,
    proposed_by text not null,
    data_json text not null
);

create table if not exists critiques (
    id text primary key,
    session_id text not null,
    reviewer text not null,
    data_json text not null
);

create table if not exists proposals (
    id text primary key,
    session_id text not null,
    proposed_by text not null,
    data_json text not null
);

create table if not exists votes (
    id text primary key,
    session_id text not null,
    voter text not null,
    proposal_id text not null,
    data_json text not null
);

create table if not exists session_events (
    id integer primary key autoincrement,
    session_id text not null,
    event_type text not null,
    payload_json text not null,
    created_at text not null
);

create table if not exists claims (
    id text primary key,
    session_id text not null,
    proposed_by text not null,
    data_json text not null
);

create table if not exists evidence (
    id text primary key,
    session_id text not null,
    proposed_by text not null,
    data_json text not null
);

create table if not exists assessments (
    id text primary key,
    session_id text not null,
    assessor text not null,
    data_json text not null
);

create table if not exists claim_evidence_links (
    claim_id text not null,
    evidence_id text not null,
    session_id text not null,
    link_type text not null,
    primary key (claim_id, evidence_id)
);
