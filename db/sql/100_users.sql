create table users (
    id bigint not null primary key,

    username varchar not null unique,
    hash varchar not null,

    email varchar unique,
    email_verified boolean not null default false,

    totp_enabled boolean not null default false,
    totp_algorithm smallint,
    totp_secret varchar,
    totp_step smallint,
    totp_digits smallint
)