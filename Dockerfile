FROM rust:1.70

WORKDIR /usr/src/myapp
COPY . .

RUN cargo build --release

CMD ["./target/release/aslan-core"]