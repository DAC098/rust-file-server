create table fs_items (
    id bigint not null primary key,
    item_type smallint not null,
    parent bigint,
    users_id bigint not null,

    directory varchar not null,
    basename varchar not null,

    item_size bigint default 0,

    created timestamp with time zone not null,
    modified timestamp with time zone,

    item_exists boolean default true,

    user_data json not null,

    is_root boolean not null default false,

    constraint users_id_fk foreign key (users_id) references users (id),
    constraint perent_fk foreign key (parent) references fs_items (id)
)