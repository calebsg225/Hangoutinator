-- Add migration script here
-- Create Todo Table
CREATE TABLE todo (
	task TEXT NOT NULL,
	user_id INTEGER NOT NULL,
	t timestamptz NOT NULL
);

-- individual meetup group events
CREATE TABLE meetupEvents (
	id uuid NOT NULL,
	PRIMARY KEY (id),
	meetupEventId TEXT NOT NULL UNIQUE,
	-- NOTE: the discord event id may not be unique due to duplicates
	discordEventId TEXT NOT NULL,
	-- the meetup groups the event belongs to
	meetupGroupId uuid NOT NULL,
	eventHash binary(128) NOT NULL,
	-- keep track of duplicate events across meetup groups
	duplicateEventHash binary(64) NOT NULL,
	-- keep track of end time to be removed from db
	endTime timestamptz NOT NULL,
);

-- meetup groups to keep track of
CREATE TABLE meetupGroups (
	id uuid NOT NULL,
	PRIMARY KEY (id),
	groupName TEXT NOT NULL UNIQUE,
	groupId binary(16),
);

-- discord guild information
CREATE TABLE guilds (
	id uuid NOT NULL,
	PRIMARY KEY (id),
	guildId binary(32) NOT NULL UNIQUE,
	welcomeRoleId binary(32) NOT NULL,
	welcomeChannelId binary(32) NOT NULL,
	adminRoleId binary(32) NOT NULL,
);

-- Create 'linker' table between `meetupGroups` and `guilds` tables
CREATE TABLE meetupGroupsGuilds (
	id uuid NOT NULL,
	PRIMARY KEY (id),
	guildId uuid NOT NULL,
	meetupGroupId uuid NOT NULL,
);
