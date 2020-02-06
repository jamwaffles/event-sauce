table! {
    events (id) {
        id -> Uuid,
        sequence_number -> Int4,
        event_type -> Varchar,
        entity_type -> Varchar,
        entity_id -> Uuid,
        data -> Nullable<Jsonb>,
        session_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        purger_id -> Nullable<Uuid>,
        purged_at -> Nullable<Timestamptz>,
    }
}
