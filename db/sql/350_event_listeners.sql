create table event_listeners (
    id uuid not null,
    
    event_name varchar not null,
    endpoint varchar not null,

    ref_table varchar not null,
    ref_id bigint not null
)