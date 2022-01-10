create table email_verificiations (
    users_id integer not null primary key,
    key_id varchar not null unique,
    issued timestamp with time zone not null,

    constraint users_id_fk foreign key (users_id) references users (id)
);