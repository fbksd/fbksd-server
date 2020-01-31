docker run                                                          \
    --name fbksd-ci                                                 \
    -d                                                              \
    -v $FBKSD_DATA_ROOT/tmp:/mnt/fbksd-data/tmp                     \
    -v $FBKSD_DATA_ROOT/scenes:/mnt/fbksd-data/scenes:ro            \
    -v $FBKSD_DATA_ROOT/renderers:/mnt/fbksd-data/renderers:ro      \
    -v $FBKSD_DATA_ROOT/config.json:/mnt/fbksd-data/config.json:ro  \
    -v /var/lock/fbksd.lock:/var/lock/fbksd.lock:ro                 \
    --env FBKSD_DATA_ROOT=$FBKSD_DATA_ROOT                          \
    --env FBKSD_SERVER_ADDR=fbksd-server:8096                       \
    --network fbksd-net                                             \
    fbksd-ci