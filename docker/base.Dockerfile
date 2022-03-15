FROM rust:1.59-alpine3.15 as base

EXPOSE 10087/tcp
EXPOSE 10087/udp

RUN apk add --no-cache musl-dev

ENV APP_DIR=/cs4850-final-project

# Copy all project files (adhering to .dockerignore) into the application directory
COPY ./ ./${APP_DIR}

WORKDIR ${APP_DIR}

RUN cargo build --release
