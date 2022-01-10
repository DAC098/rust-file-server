create table user_sessions (
    users_id bigint not null,
    
    token uuid not null,

    dropped boolean not null default false,

    issued_on timestamp with time zone not null,
    expires timestamp with time zone not null,

    constraint users_id_fk foreign key (users_id) references users (id)
)