docker run                                  \
    --name fbksd-server                     \
    --restart unless-stopped                \
    -d                                      \
    -v $FBKSD_DATA_ROOT:/mnt/fbksd-data     \
    -v /var/lock:/var/lock                  \
    --env FBKSD_DATA_ROOT=$FBKSD_DATA_ROOT  \
    --env FBKSD_WWW_USER="fbksd-ci"         \
    --env FBKSD_WWW_GROUP="fbksd-ci"        \
    --network fbksd-net                     \
    fbksd-server