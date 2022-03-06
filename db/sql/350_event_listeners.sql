create table event_listeners (
    id uuid primary key not null,
    
    event_name varchar not null,
    endpoint varchar not null,

    ref_table varchar not null,
    ref_id bigint not null,

    users_id bigint not null,

    constraint users_id_fk foreign key (users_id) references users (id)
)