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
    - [Managing Event Syncing](#managing-event-syncing)

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
 - `Create Events`
 - `Manage Events`
 - `View Channels`

Scroll to the bottom of the page. Go to the URL generated and follow the prompts.

### Setting up the environment

Clone the repo, then `cd` into it.

Make sure to create the `.env` file following `.env.example`.

Grab your bots token from your bots application page by selecting `Bot` from the sidebar
and scrolling down to `TOKEN`.

### Running the bot

#### Prerequisites

- Rust and Cargo
- Docker

Run `make hangoutinator` in your terminal from the projects main directory to build the bot and run the application. This may take a few minutes the first time. 

The bot and postgres db should now be running.

A database visualization tool is also included in the application, which should now be running locally on port `6969`. To log in:

- `System`: `PostgreSQL`
- `Server`: `database`
- `Username`: [PGUSER_FROM_DOT_ENV]
- `Password`: [PGPASSWORD_FROM_DOT_ENV]
- `Database`: [PGDATABASE_FROM_DOT_ENV]

### Useful commands

|Command|Function|
|-|-|
|`make hangoutinator`|builds bot and runs the entire application|
|`make release`|builds the bot image from the dockerfile|
|`make update`|takes down bot, re-builds, brings bot back online|
|`make migrate`|run migration on postgres db|
|`make logs`|prints out bot logs|

## Features

### Welcoming Verified Members

### Managing Event Syncing
