# hangoutinator

![](./images/doofenshmirtz.webp)

## Overview
- [Usage](#usage)
    - [Create the app](#create-the-app)
    - [Server invitation](#server-invitation)
    - [Setting up the environment](#setting-up-the-environment)
    - [Running the bot](#running-the-bot)
    - [Other useful commands](#other-useful-commands)
- [Features](#features)
    - [Welcoming Verified Members](#welcoming-verified-members)

## Usage

This project is designed to be run on a debian-based linux server, and is therefore being tested
only on a debian-based linux machine for the moment.

### Create the app

Create a new discord application [here](https://discord.com/developers/applications).

For the [Welcome Role](#welcoming-verified-members) feature to work, `Server Members Intent` must be
toggled on. Do this from your bot application page by selecting `Bot` on the sidebar and scrolling to
`SERVER MEMBERS INTENT`, then flipping on the toggle.

### Server invitation

To add your bot to a server, on your bot application page, select `OAuth2` on the sidebar. Scroll to the `OAuth URL Generator`. 
In the `SCOPES` panel, toggle the following:
 - `bot`
 - `applications.commands`

A `BOT PERMISSIONS` panel will appear below. Toggle the following from this panel:
 - `Send Messages`

Scroll to the bottom of the page. Go to the URL generated and follow the prompts.

### Setting up the environment

Clone the repo, then `cd` into it.

Make sure to create the `.env` file following `.env.example`.

Grab your bots token from your bots application page by selecting `Bot` from the sidebar
and scrolling down to `TOKEN`.

The desired Verified Role Id and Welcome Channel Id can be copied from your discord server.

### Running the bot

Run `make` in your terminal from the projects main directory to build the docker image. This may take a few minutes the first time. 
Once complete, run `make run` to create and start the docker container from the newly created image.

The bot should now be running.

### Other useful commands

|Command|Function|
|-|-|
|`make`|builds the docker image|
|`make create`|creates the docker container from the image|
|`make start`|starts the docker container|
|`make run`|combined `make create` and `make start`|
|`make stop`|stop the running docker container|
|`make destroy`|destroy the stopped docker container|
|`make logs`|prints out logs from the docker container|

## Features

### Welcoming Verified Members

TODO: make commands for selecting roles and channels for this feature from discord

** does not yet support mutliple guilds
