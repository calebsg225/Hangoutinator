-- persistantly keep track of which welcome message should be used next
ALTER TABLE guilds
ADD message_index INT;
