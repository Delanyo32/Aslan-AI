FROM rust:1.65

ENV REDIS_URL=redis://localhost:6379/0

WORKDIR /usr/src/myapp
COPY . .

RUN cargo build --release

CMD ["./target/release/aslan-core"]