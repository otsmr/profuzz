#!/bin/bash

# Mount /dev/urandom into the dev directory
mount --bind /dev/ /workdir/root/dev
# mount --bind /dev/ /share/root/dev

# This will start the ssh daemon
chroot root /qemu-mipsel-static /usr/bin/dropbear -p 22 -r /var/tmp/dropbear/dropbear_rsa_host_key -d /var/tmp/dropbear/dropbear_dss_host_key -A /var/tmp/dropbear/dropbearpwd

# This will start the fuzzing target
chroot root /qemu-mipsel-static /usr/bin/tmpd &
sleep infinity
