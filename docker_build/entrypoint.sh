#!/bin/sh

if [ ! -f /data/jellycord.yaml ]; then
	echo "discord_token: \"<>\"\ncommand_prefix: '~'" > /data/jellycord.yaml
	echo "Please enter the discord_token into the config file and restart the container."
	cd /data && SETUP=1 /usr/local/cargo/bin/jellycord
	chown -R $UID:$GID /data
else
	cd /data 

	# this step is necessary so that the permissions for the db can be changed before running jellycord
	if [ ! -f /data/jellycord.sqlite ]; then
		echo "Creating database..."
		SETUP=1 /usr/local/cargo/bin/jellycord
	fi

	chown -R $UID:$GID /data
	/usr/local/cargo/bin/jellycord
fi
