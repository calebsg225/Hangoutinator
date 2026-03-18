-- Give the option to log info to a discord channel
ALTER TABLE guilds
ADD logging_channel_id NUMERIC(20),
ADD logging_level INT;
