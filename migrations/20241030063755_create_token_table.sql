-- Add migration script here
CREATE TABLE tokens (
    user_id uuid PRIMARY KEY REFERENCES users (user_id),
    token TEXT NOT NULL UNIQUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);