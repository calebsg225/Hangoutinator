-- Add migration script here
-- Create Todo Table
CREATE TABLE todo (
	task TEXT NOT NULL,
	user_id INTEGER NOT NULL,
	t timestamptz NOT NULL
);

-- individual meetup group events
CREATE TABLE meetup_events (
	id uuid NOT NULL,
	PRIMARY KEY (id),
	meetup_event_id TEXT NOT NULL UNIQUE,
	-- the meetup groups the event belongs to
	meetup_group_id uuid NOT NULL,
	event_hash TEXT NOT NULL,
	-- keep track of duplicate events across meetup groups
	duplicate_event_hash TEXT NOT NULL,
	-- keep track of end time to be removed from db
	end_time timestamptz NOT NULL
);

-- all discord events from all guilds bot is active in
CREATE TABLE discord_events (
	id uuid NOT NULL,
	PRIMARY KEY (id),
	discord_event_id NUMERIC(20) NOT NULL UNIQUE
);

-- meetup groups to keep track of
CREATE TABLE meetup_groups (
	id uuid NOT NULL,
	PRIMARY KEY (id),
	group_name TEXT NOT NULL UNIQUE,
	group_id NUMERIC(10)
);

-- discord guild information
CREATE TABLE guilds (
	id uuid NOT NULL,
	PRIMARY KEY (id),
	guild_id NUMERIC(20) NOT NULL UNIQUE,
	welcome_role_id NUMERIC(22),
	welcome_channel_id NUMERIC(20),
	admin_role_id NUMERIC(22)
);

-- Create 'linker' table between `meetup_groups` and `guilds` tables
CREATE TABLE meetup_groups_guilds (
	id uuid NOT NULL,
	PRIMARY KEY (id),
	guild_id uuid NOT NULL,
	meetup_group_id uuid NOT NULL
);

-- Create 'linker' table between `discord_events` and `meetup_events` tables
CREATE TABLE discord_events_meetup_events (
	id uuid NOT NULL,
	PRIMARY KEY (id),
	discord_event_id uuid NOT NULL,
	meetup_event_id uuid NOT NULL
);
