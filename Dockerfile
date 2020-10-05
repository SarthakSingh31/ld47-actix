FROM archlinux:latest

WORKDIR /app/
ADD ./target/release/ld47-actix ./
ADD ./cert.pem ./
ADD ./key.pem ./

CMD MODE="SSL" ./ld47-actix