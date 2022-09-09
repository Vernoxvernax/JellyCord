## **RIP @DmDb**

___

### here is my replacement.

**Delete your database for this new release**

don't use it

currently only jellyfin

```
echo "discord_token: <DISCORD_TOKEN>" > jellycord.yaml
cargo run
```


**HOST YOURSELF!!!**


Changelog:

* added env variable "SETUP" (checking for "1"). Which will result in only the database being created.
* added basic Dockerfile and entrypoint.sh

This makes it possible to run it in a docker and chown the file for non-root access.


##### now I hate async even more