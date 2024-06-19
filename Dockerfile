FROM rust:1.79.0

WORKDIR /usr/src/raft

COPY . .

RUN cargo install --path .

RUN cargo build

CMD ["target/release/alexandria"]
