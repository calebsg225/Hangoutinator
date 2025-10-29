-- Add migration script here
-- Create Todo Table
CREATE TABLE todo (
	task TEXT NOT NULL,
	user_id INTEGER NOT NULL,
	t timestamptz NOT NULL
);
