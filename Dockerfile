FROM rust:1.65


WORKDIR /usr/src/myapp
COPY . .

RUN cargo build --release

CMD ["./target/release/aslan-core"]