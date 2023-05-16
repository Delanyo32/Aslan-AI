FROM rust:1.69


WORKDIR /usr/src/myapp
COPY . .

RUN cargo build --release

CMD ["./target/release/aslan-core"]