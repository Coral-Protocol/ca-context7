ARG RUST_VERSION=1.89
FROM rust:${RUST_VERSION}-bullseye AS builder
WORKDIR /app

COPY agent agent

# Ssl required for building
RUN apt-get update
RUN apt-get install -y libssl-dev

RUN cargo install --path agent

FROM debian:bullseye-slim
WORKDIR /app

COPY mcp mcp

ARG NODE_VERSION=20
ENV NVM_DIR=/root/.nvm

# Ssl also required for running...
RUN apt update && apt install -y libssl-dev curl

# nvm/npm/node https://github.com/nvm-sh/nvm?tab=readme-ov-file#installing-in-docker
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash
RUN bash -c "source $NVM_DIR/nvm.sh && nvm install $NODE_VERSION && npm --prefix mcp install"

RUN echo "#!/bin/bash" >> run.sh && \
    echo "source $NVM_DIR/nvm.sh" >> run.sh && \
    echo "node /app/mcp/node_modules/@upstash/context7-mcp/dist/index.js" >> run.sh && \
    chmod +x run.sh

COPY --from=builder /usr/local/cargo/bin/context7 /usr/local/bin/context7

CMD ["context7"]
