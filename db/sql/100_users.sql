create table users (
    id bigint not null primary key,

    username varchar not null unique,
    hash varchar not null,

    email varchar unique,
    email_verified boolean not null default false
)