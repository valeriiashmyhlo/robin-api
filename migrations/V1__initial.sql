CREATE TABLE users (
    id UUID PRIMARY KEY,
    username VARCHAR(255) NOT NULL,
    token UUID NOT NULL,
    password VARCHAR(255) NOT NULL
);
