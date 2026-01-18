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

**IMPORTANT: The version of the application on this branch will only work as intended when running on a single discord guild. Multi-guild support is not yet operational.**

This project is designed to be run on a debian-based linux server, and is therefore being tested
only on a debian-based linux machine for the moment.

### Create the app

Create a new discord application [here](https://discord.com/developers/applications).

For the [Welcome Role](#welcoming-verified-members) feature to work, `Server Members Intent` must be
toggled on. Do this from your bot application page by selecting `Bot` on the sidebar and scrolling to
`Server Members Intent`, then flipping on the toggle.

While here, also toggle on `Message Content Intent`.

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

**IMPORTANT: The commands in `makefile` do not prepend `docker` commands with `sudo`. Depending on your setup, `sudo` is required to run `docker` commands.**

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

For this feature to work, you need to set a `welcome channel` and a `welcome role`. Do this with the built-in slash command `/set`:
- `/set welcome_channel`
- `/set welcome_member_role`

When a member has been verified and has consequently been given the `welcome role`, the bot will send that user a welcome message in the set `welcome channel`.

### Managing Meetup Event Syncing

**IMPORTANT: Single guild use only. The first guild the bot is added to is the only guild this feature will work for.**

For meetup event syncing, use the `/meetup` slash command.

When you want to track a meetup group, use `/meetup track`. The group name inputed must be the url name of the meetup group, eg. in `https://meetup.com/my-meetup-group/events`, the name to input is `my-meetup-group`.

To untrack a meetup group, use `/meetup untrack` in the same way.

View tracked meetup groups with `/meetup list`.

Once you have edited the tracked group list to your liking, you can use `/meetup resync` to manually refetch meetup event data and resync with discord. If you do not do this, data will be refetched automatically at the next resync interval (default is once every hour).

If meetup group data cannot be fetched, it will be skipped and an error will be sent in the logs (use `make logs` in the terminal).
