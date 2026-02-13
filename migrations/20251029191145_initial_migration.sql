-- individual meetup group events
CREATE TABLE meetup_events (
	meetup_event_id VARCHAR(15) NOT NULL UNIQUE,
	meetup_group_id VARCHAR(15) NOT NULL,
	meetup_group_name VARCHAR(100) NOT NULL,
	title TEXT NOT NULL,
	description TEXT NOT NULL,
	location TEXT NOT NULL,
	-- detect updates to a meetup event
	event_hash NUMERIC(20) NOT NULL,
	-- duplicate events across meetup groups
	duplicate_event_hash NUMERIC(20) NOT NULL,
	-- groups meetup events that repeat weekly
	weekly_collection_hash NUMERIC(20) NOT NULL,
	-- TODO: monthly collection hash?
	-- time of event
	start_time timestamptz NOT NULL,
	end_time timestamptz NOT NULL,
	-- time of last attempted sync.
	last_synced timestamptz NOT NULL,

	PRIMARY KEY (meetup_event_id)
);

-- all discord events from all guilds bot is active in
CREATE TABLE discord_events (
	discord_event_id NUMERIC(20) NOT NULL UNIQUE,
	duplicate_event_hash NUMERIC(20) NOT NULL,
	-- the discord event is tracking all meetup events with this collection hash
	collection_hash NUMERIC(20) NOT NULL,
	guild_id NUMERIC(20) NOT NULL,
	start_time timestamptz NOT NULL,
	end_time timestamptz NOT NULL,
	-- TODO: last_synced?
	PRIMARY KEY (discord_event_id)
);

-- meetup groups to keep track of
CREATE TABLE meetup_groups (
	group_name VARCHAR(100) NOT NULL UNIQUE,
	PRIMARY KEY (group_name)
);

-- discord guild information
CREATE TABLE guilds (
	guild_id NUMERIC(20) NOT NULL UNIQUE,
	welcome_role_id NUMERIC(22),
	welcome_channel_id NUMERIC(20),
	access_role_id NUMERIC(22),
	PRIMARY KEY (guild_id)
);

-- Create 'linker' table between `meetup_groups` and `guilds` tables
CREATE TABLE meetup_groups_guilds (
	group_name VARCHAR(100) NOT NULL,
	guild_id NUMERIC(20) NOT NULL,
	PRIMARY KEY (group_name, guild_id),
	FOREIGN KEY (group_name) REFERENCES meetup_groups(group_name) ON DELETE CASCADE,
	FOREIGN KEY (guild_id) REFERENCES guilds(guild_id) ON DELETE CASCADE
);

-- Create 'linker' table between `discord_events` and `meetup_events` tables
CREATE TABLE discord_events_meetup_events (
	discord_event_id NUMERIC(20) NOT NULL,
	meetup_event_id VARCHAR(15) NOT NULL,
	PRIMARY KEY (discord_event_id, meetup_event_id),
	FOREIGN KEY (discord_event_id) REFERENCES discord_events(discord_event_id) ON DELETE CASCADE,
	FOREIGN KEY (meetup_event_id) REFERENCES meetup_events(meetup_event_id) ON DELETE CASCADE
);
