-- include `created_time` in `meetup_events`
ALTER TABLE meetup_events
ADD created_time timestamptz;

UPDATE meetup_events
SET created_time='2020-01-01 01:00:00+04'
WHERE created_time IS NULL;

ALTER TABLE meetup_events
ALTER COLUMN created_time SET NOT NULL;
