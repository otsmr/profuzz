FROM rust:latest
RUN mkdir /workdir
WORKDIR /workdir
COPY Cargo.toml  ./
COPY src ./src
CMD ["cargo", "run"]
