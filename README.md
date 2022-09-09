Currently only jellyfin

```
echo "discord_token: <DISCORD_TOKEN>" > jellycord.yaml
cargo run --release
```

For docker deployment:

```
git clone ..., cd ...
sudo docker build -t jellycord .
```

`docker-compose.yml`
```
version: '3.7'
services:
  jellycord:
    image: jellycord
    container_name: "JellyCord"
    volumes:
      - ./data:/data
    environment:
      - UID=1000
      - GID=1000
```

```
sudo docker-compose up
...
```


**HOST YOURSELF!!!**


Changelog:

* added env variable "SETUP" (checking for "1"). Which will result in only the database being created.
* added basic Dockerfile and entrypoint.sh

This makes it possible to run it in a docker and chown the file for non-root access.


##### now I hate async even more
