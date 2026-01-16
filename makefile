IMAGE_NAME = hangoutinator
CONTAINER_NAME = hangbot1

hangoutinator: release
	docker compose up -d
	make migrate

migrate:
	cargo sqlx migrate run

release:
	docker compose build bot

update:
	make release
	docker compose down bot
	docker compose up bot -d
	make logs

db: 
	docker compose down database
	docker compose up database -d
	make migrate

logs:
	docker compose logs bot -f
