CREATE TABLE chats (
    id uuid PRIMARY KEY
);

CREATE TABLE messages (
    id uuid PRIMARY KEY,
    user_id uuid not null,
    chat_id uuid not null,
    content text not null
);
