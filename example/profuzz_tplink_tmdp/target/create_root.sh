# Commands used to create the /root directory

# docker run -p 22:22 -v "$(pwd):/share" -d --privileged=true --name tplink tplink
# docker exec -it tplink /bin/bash

mount --bind /dev/ squashfs-root/dev

cp /etc/passwd squashfs-root/var/passwd
mkdir -p squashfs-root/var/tmp/dropbear/
rm squashfs-root/var/tmp/dropbear/dropbear_dss_host_key
echo "username:admin" >squashfs-root/var/tmp/dropbear/dropbearpwd
echo "password:21232f297a57a5a743894a0e4a801fc3" >>squashfs-root/var/tmp/dropbear/dropbearpwd
chroot squashfs-root /qemu-mipsel-static /usr/bin/dropbearkey -t rsa -f /var/tmp/dropbear/dropbear_rsa_host_key
chroot squashfs-root /qemu-mipsel-static /usr/bin/dropbearkey -t dss -f /var/tmp/dropbear/dropbear_dss_host_key

chroot squashfs-root /qemu-mipsel-static /usr/bin/dropbear -p 22 -r /var/tmp/dropbear/dropbear_rsa_host_key -d /var/tmp/dropbear/dropbear_dss_host_key -A /var/tmp/dropbear/dropbearpwd
chroot squashfs-root /qemu-mipsel-static /usr/bin/tmpd