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
	docker compose down bot
	make release
	docker compose up bot -d

db: 
	docker compose down database
	docker compose up database -d
	make migrate

logs:
	docker compose logs bot
