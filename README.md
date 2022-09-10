## JellyCord

**Jellyfin and Emby supported**

___
### Binary-run:

```
echo "discord_token: <DISCORD_TOKEN>" > jellycord.yaml
cargo run --release
```
___

### Docker deployment:

```
git clone <git-url>
sudo docker build -t jellycord <git-dir>
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
sudo docker-compose up -d
```
___

### NOTES:

* Make sure to edit the config file.
* The default command prefix is '~'

___

[**Changelog**](https://github.com/Vernoxvernax/JellyCord/blob/main/Changelog.md)


Warning:
I highly encourage you host this yourself. Anyone with the sqlite database can access your media with admin like permissions.
