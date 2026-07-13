FROM rust:latest

RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 python3-pip python3-pytest git curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install uv --quiet --break-system-packages

RUN curl -fsSL https://mise.jdx.dev/install.sh | MISE_INSTALL_DIR=/usr/local/bin sh

RUN cargo install opencode-ai

RUN useradd -m -u 1000 agent

RUN mkdir /workspace && chown agent:agent /workspace
WORKDIR /workspace

USER agent

ENTRYPOINT ["opencode"]
