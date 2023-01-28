## JellyCord

**supports Jellyfin and Emby**

___

### Docker deployment:

```
git clone <git-url>
sudo docker build -t jellycord <git-dir>
```

`docker-compose.yml`
```yml
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

### Example:

<img src="https://i.imgur.com/wcrIerK.png" height="501"/>

___
### NOTES:

* Make sure to edit the config file.
* The default command prefix is '~'
* Episodes are not being sent individually to your channel if the series object is new as well.
* Banners:
  * Are fetched from your server directly, so if the domain you've provided to the bot isn't publicly accessible, then pictures will fail (I think; well at least for users outside your network; but then what's the point of this anyway lol).
  * Images on Emby and Jellyfin are retrieved using the API key, which is part of the URL of the image. Please be careful when sharing access to these messages as they grant access to all content on the media server.
* If you got any recommendations for features or the Image-API-Key-Problem please let me know asap.
___

[**Changelog**](https://github.com/Vernoxvernax/JellyCord/blob/main/Changelog.md)


Warning:
Do not allow anyone else to host this for your media-server, under any circumstances!
