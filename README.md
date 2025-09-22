# hangoutinator

![](./images/doofenshmirtz.webp)

## Usage

### Create the app

TODO

- `Server Members Intent` must be toggled on:

### Setting up the environment

Before advancing further, make sure to create the `.env` file following `.env.example`.

The desired Verified Role Id and Welcome Channel Id can be copied from the discord server.

### Running the bot

Build the docker image:
```
make
```

Create the docker container:
```
make create
```

Start the container:
```
make start
```

Combined Create/Start:
```
make run
```

Stop the container:
```
make stop
```

Destroy the container:
```
make destroy
```

## Features

### Welcoming Verified Users

TODO

** does not yet support mutliple guilds
