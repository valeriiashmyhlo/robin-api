CREATE TABLE users (
    id uuid PRIMARY KEY,
    username varchar(255) not null,
    token uuid not null,
    password varchar(255) not null
);
