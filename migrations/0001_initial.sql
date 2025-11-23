CREATE TABLE users (
    id integer primary key generated always as identity,
    username text not null unique,
    password_hash text not null
);

CREATE TABLE sessions (
    token text primary key,
    user_id integer not null references users(id),
    expires timestamptz not null
);

CREATE TABLE pastes (
    id text primary key,
    user_id integer not null references users(id),
    visibility text not null check (visibility in ('public', 'unlisted', 'private'))
);

CREATE TABLE paste_files (
    paste_id text not null references pastes(id) on delete cascade,
    filename text not null,
    content text not null,
    primary key (paste_id, filename)
);
