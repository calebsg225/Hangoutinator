start: release
	docker compose up database -d
	make migrate
	docker compose up -d
	make logs

stop:
	docker compose down

migrate:
	cargo sqlx migrate run

release:
	docker compose build bot

update: release
	docker compose down bot
	docker compose up bot -d
	make logs

# bring down application and remove all data from db
purge: 
	docker compose down -v

logs:
	docker compose logs bot -f
