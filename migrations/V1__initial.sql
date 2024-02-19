CREATE TABLE users (
    id serial PRIMARY KEY,
    first_name varchar(255) not null,
    token uuid not null,
    password varchar(255) not null
);