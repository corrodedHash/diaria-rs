FROM rust:latest

RUN apt-get update && apt-get install -y --no-install-recommends \
    git curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://mise.jdx.dev/install.sh | MISE_INSTALL_PATH=/usr/local/bin/mise sh

ENV MISE_DATA_DIR=/opt/mise
ENV PATH=/opt/mise/shims:$PATH

COPY mise.toml /tmp/mise.toml
RUN cd /tmp && mise trust mise.toml && mise install

RUN OPENCODE_INSTALL_DIR=/usr/local/bin \
    curl -fsSL https://opencode.ai/install | bash

RUN useradd -m -u 1000 agent

RUN mkdir /workspace && chown agent:agent /workspace
RUN mkdir -p /home/agent/.local/share/opencode && chown agent:agent /home/agent/.local/share/opencode
WORKDIR /workspace

USER agent

ENTRYPOINT ["opencode"]
