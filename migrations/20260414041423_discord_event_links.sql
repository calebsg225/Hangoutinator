-- provide a channel for sending links for recent discord events
ALTER TABLE guilds
ADD event_links_channel_id NUMERIC(20);
