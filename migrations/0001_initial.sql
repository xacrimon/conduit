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
    id integer primary key generated always as identity,
    external_id uuid not null unique
);

CREATE TABLE paste_revisions (
    paste_id integer not null references pastes(id),
    revision_seq integer not null,
    revision_id integer not null unique,
    primary key (paste_id, revision_seq)
);

CREATE TABLE paste_files (
    revision_id integer not null references paste_revisions(revision_id),
    filename text not null,
    content text not null,
    primary key (revision_id, filename)
);
