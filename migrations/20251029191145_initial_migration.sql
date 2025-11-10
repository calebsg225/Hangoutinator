-- Add migration script here
-- Create Todo Table
CREATE TABLE todo (
	task TEXT NOT NULL,
	user_id INTEGER NOT NULL,
	t timestamptz NOT NULL
);

-- individual meetup group events
CREATE TABLE meetup_events (
	meetup_event_id NUMERIC(12) NOT NULL UNIQUE,
	PRIMARY KEY (meetup_event_id),
	-- the meetup groups the event belongs to
	meetup_group_id NUMERIC(12) NOT NULL,
	event_hash TEXT NOT NULL,
	-- keep track of duplicate events across meetup groups
	duplicate_event_hash TEXT NOT NULL,
	-- keep track of end time to be removed from db
	end_time timestamptz NOT NULL
);

-- all discord events from all guilds bot is active in
CREATE TABLE discord_events (
	discord_event_id NUMERIC(20) NOT NULL UNIQUE,
	PRIMARY KEY (discord_event_id)
);

-- meetup groups to keep track of
CREATE TABLE meetup_groups (
	group_id NUMERIC(12) NOT NULL UNIQUE,
	PRIMARY KEY (group_id),
	group_name TEXT NOT NULL UNIQUE
);

-- discord guild information
CREATE TABLE guilds (
	guild_id NUMERIC(20) NOT NULL UNIQUE,
	PRIMARY KEY (guild_id),
	welcome_role_id NUMERIC(22),
	welcome_channel_id NUMERIC(20),
	admin_role_id NUMERIC(22)
);

-- Create 'linker' table between `meetup_groups` and `guilds` tables
CREATE TABLE meetup_groups_guilds (
	id NUMERIC(32) NOT NULL UNIQUE,
	PRIMARY KEY (id),
	guild_id NUMERIC(20) NOT NULL,
	meetup_group_id NUMERIC(12) NOT NULL
);

-- Create 'linker' table between `discord_events` and `meetup_events` tables
CREATE TABLE discord_events_meetup_events (
	id NUMERIC(32) NOT NULL UNIQUE,
	PRIMARY KEY (id),
	discord_event_id NUMERIC(20) NOT NULL,
	meetup_event_id NUMERIC(12) NOT NULL
);
