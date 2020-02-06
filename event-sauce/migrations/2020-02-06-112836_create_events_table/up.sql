create extension if not exists "uuid-ossp";

create table if not exists events(
    id uuid primary key default uuid_generate_v4(),
    sequence_number serial,
    event_type varchar(64) not null,
    entity_type varchar(64) not null,
    entity_id uuid not null,
    data jsonb, -- This field is null if the event is purged, in such case purged_at and purger_id won't be null either.
    session_id uuid null,
    created_at timestamp with time zone not null,
    purger_id uuid null,
    purged_at timestamp with time zone null
);
