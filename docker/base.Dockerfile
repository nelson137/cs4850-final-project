FROM rust:1.56-slim-bullseye as base

EXPOSE 10087/tcp
EXPOSE 10087/udp

ENV APP_DIR=/cs4850-final-project

# Copy all project files (adhering to .dockerignore) into the application directory
COPY ./ ./${APP_DIR}

WORKDIR ${APP_DIR}

RUN cargo build --release
