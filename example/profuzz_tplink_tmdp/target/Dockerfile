# Use the official Ubuntu base image
FROM ubuntu:latest

# Set environment variables
ENV DEBIAN_FRONTEND=noninteractive

# Install necessary packages
RUN apt-get update
RUN apt-get install -y gcc make dropbear zlib1g-dev net-tools curl vim gcc-mipsel-linux-gnu qemu-user-static gdb-multiarch

# Expose the ports
EXPOSE 22

RUN mkdir /workdir
WORKDIR /workdir

COPY ./root /workdir/root
COPY entry.sh /workdir/entry.sh
RUN chmod +x /workdir/entry.sh
RUN cp /usr/bin/qemu-mipsel-static /workdir/root
RUN chmod +x /workdir/root/

# RUN echo "#!/bin/bash\n\nsleep infinity" >> /sleep.sh
# RUN chmod +x /sleep.sh
# ENTRYPOINT [ "/sleep.sh" ]

ENTRYPOINT [ "/workdir/entry.sh" ]
