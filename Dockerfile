FROM rust:1.67

WORKDIR /usr/src/raft

COPY . .

RUN cargo install --path .

RUN cargo build

CMD ["target/release/alexandria"]
