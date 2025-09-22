IMAGE_NAME = hangoutinator
CONTAINER_NAME = hangbot1

default: build

build:
	sudo docker build --tag ${IMAGE_NAME} --file Dockerfile .
	make prune

start:
	sudo docker start ${CONTAINER_NAME}

create:
	sudo docker create -it --name ${CONTAINER_NAME} --env-file ./.env ${IMAGE_NAME}

run: create start

prune:
	sudo docker image prune -f

stop:
	sudo docker kill ${CONTAINER_NAME}

destroy:
	sudo docker rm ${CONTAINER_NAME}

logs:
	sudo docker logs ${CONTAINER_NAME}
