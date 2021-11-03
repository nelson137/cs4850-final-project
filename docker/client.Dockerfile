FROM chat-boat/base

ENTRYPOINT "/${APP_DIR}/target/release/chat-client" --

CMD []
