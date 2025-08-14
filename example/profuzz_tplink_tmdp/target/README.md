# TP-Link target example

## Real device

Admin password: Admin!

```bash
# /opt/homebrew/bin/ssh -v -N -oHostKeyAlgorithms=+ssh-dss -o "UserKnownHostsFile=/dev/null" -o "StrictHostKeyChecking=no" -L 20002:192.168.0.1:20002 -p 22 admin@192.168.0.1
/opt/homebrew/bin/ssh -v -N  -o "UserKnownHostsFile=/dev/null" -o "StrictHostKeyChecking=no" -L 20002:127.0.0.1:20002 -p 22 admin@192.168.0.1
```


## Emulation


Files are downloaded from the [Support](https://www.tp-link.com/de/support/download/tl-wr902ac/#Firmware) page from TP-Link.

```plain
TL-WR902AC(EU)_V4_240903
Datum der Veröffentlichung: 2025-01-26
```

The files in root where extracted with the following commands:

```sh
binwalk --dd=".*" firmware.bin
fakeroot -s f.dat unsquashfs -d squashfs-root 160200
```

## Getting started


1. Build the Docker container
```sh
cd example/tplink/
docker build -t tplink .
```
2. Create a new container
```sh
docker run -p 22:22 -v "$(pwd):/share" -d --privileged=true --name tplink tplink
```
The `--privileged=true` flag is required so that we can mount the `/dev` folder into our fake root.

3. Start two shells inside of the container
```sh
docker exec -it tplink /bin/bash

# This will start the ssh daemon
chroot root /qemu-mipsel-static /usr/bin/dropbear -p 22 -r /var/tmp/dropbear/dropbear_rsa_host_key -d /var/tmp/dropbear/dropbear_dss_host_key -F -E -A /var/tmp/dropbear/dropbearpwd

# This will start the fuzzing target
chroot root /qemu-mipsel-static /usr/bin/tmpd
```

4. Verify that the binaries are running

```sh
netstat -a
Active Internet connections (servers and established)
Proto Recv-Q Send-Q Local Address           Foreign Address         State
tcp        0      0 0.0.0.0:ssh             0.0.0.0:*               LISTEN
tcp        0      0 localhost:20002         0.0.0.0:*               LISTEN
tcp6       0      0 [::]:ssh                [::]:*                  LISTEN
```

5. Connect via the terminal

Connect to the ssh connection of the container:

```sh
ssh -v -N -oHostKeyAlgorithms=+ssh-dss -o "UserKnownHostsFile=/dev/null" -o "StrictHostKeyChecking=no" -L 20002:127.0.0.1:20002 -p 22 admin@localhost
```

You should now be able to connect to the tmdp binary:

```sh
echo -en '\x01''\x00''\x01''\x00' | nc localhost 20002 | xxd
00000000: 0100 0200                                ....
```
