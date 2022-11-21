FROM rust:1.65

LABEL "com.datadoghq.ad.check_names"='["aslan-core"]'
LABEL "com.datadoghq.ad.init_configs"='[{}]'
LABEL "com.datadoghq.ad.instances"='[{"host": "%%host%%","port":"9000"}]'

ENV REDIS_URL=redis://localhost:6379/0

WORKDIR /usr/src/myapp
COPY . .

RUN cargo build --release

CMD ["./target/release/aslan-core"]