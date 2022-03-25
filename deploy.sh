#!/bin/bash
set -e
docker run --rm -ti -v $(pwd):/app -v $(readlink -f $SSH_AUTH_SOCK):/ssh-agent -e SSH_AUTH_SOCK=/ssh-agent vulcast-cross/armv7
scp target/armv7-unknown-linux-gnueabihf/debian/vulcast-firmware_0.1.0_armhf.deb pi@raspberrypi.local:/home/pi
