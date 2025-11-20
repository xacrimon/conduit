CREATE TABLE users (
    id integer primary key generated always as identity,
    username text not null unique,
);

CREATE TABLE sessions (
    token text primary key,
    user_id integer not null references users(id),
    expires timestamptz not null
);
