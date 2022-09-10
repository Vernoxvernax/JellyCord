#!/bin/sh

if [ ! -f /data/jellycord.yaml ]; then
	echo "discord_token: \"<>\"\ncommand_prefix: '~'" > /data/jellycord.yaml
	echo "Please enter the discord_token into the config file and restart the container."
fi
cd /data && SETUP=1 /usr/local/cargo/bin/jellycord
chown -R $UID:$GID /data
/usr/local/cargo/bin/jellycord