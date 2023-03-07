## JellyCord

**supports Jellyfin and Emby**

___

### Docker deployment:

```sh
git clone <git-url>
sudo docker build -t jellycord <git-dir>
```

`docker-compose.yml`:
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

```sh
sudo docker-compose up -d
```

___

### Example:

<img src="https://i.imgur.com/L62RIoV.png" height="500"/>

___
### NOTES:

* Make sure to edit the config file.
* The default command prefix is '~'
* Series:
  * Series objects only posted by themselves and if they are new
  * Season objects only posted by themselves and if they are new
    * not if the Series object is new as well
  * Episode objects only posted by themselves
    * not if the Season object is new as well
* Banners:
  * Are fetched from your server directly, so if the domain you've provided to the bot isn't publicly accessible, then pictures will fail (I think; well at least for users outside your network; but then what's the point of this anyway lol).
  * Images on Emby and Jellyfin are retrieved using the API key, which is part of the URL of the image. Please be careful when sharing access to these messages as they grant access to all content on the media server.
* If you got any recommendations for features, or the Image-API-Key-Problem, please let me know asap.
* Some updates may bring breaking changes to the library, which will require you to reset it, and it's channel.
  * I'm obviously trying to avoid that, but sometimes it's definitely necessary.
___

[**Changelog**](https://github.com/Vernoxvernax/JellyCord/blob/main/Changelog.md)

Warning:
Do not allow anyone else to host this for your media-server, under any circumstances!
