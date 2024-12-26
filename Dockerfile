FROM rust:1.82.0-slim-bookworm AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release --target-dir /usr/src/app/target
RUN strip /usr/src/app/target/release/stakpak

FROM debian:bookworm-slim
LABEL org.opencontainers.image.source="https://github.com/stakpak/cli" \
    org.opencontainers.image.description="Stakpak CLI Tool" \
    maintainer="contact@stakpak.dev"

RUN apt-get update -y && apt-get install -y curl \
    unzip \
    git \
    apt-transport-https \
    ca-certificates \
    gnupg \
    netcat-traditional \
    wget \
    jq \
    dnsutils \
    && rm -rf /var/lib/apt/lists/*

# Install aws cli
RUN cd /tmp && \
    ARCH=$(uname -m) && \
    if [ "$ARCH" = "x86_64" ] || [ "$ARCH" = "aarch64" ]; then \
    curl "https://awscli.amazonaws.com/awscli-exe-linux-$ARCH.zip" -o "awscliv2.zip"; \
    else \
    echo "Unsupported architecture: $ARCH" && exit 1; \
    fi && \
    unzip awscliv2.zip && \
    ./aws/install && \
    rm -rf aws awscliv2.zip
# Install do cli
RUN cd /tmp && \
    ARCH=$(uname -m) && \
    DOCTL_VERSION=1.119.0 && \
    if [ "$ARCH" = "x86_64" ]; then \
    DOCTL_ARCH="amd64"; \
    elif [ "$ARCH" = "aarch64" ]; then \
    DOCTL_ARCH="arm64"; \
    else \
    echo "Unsupported architecture: $ARCH" && exit 1; \
    fi && \
    curl -LO "https://github.com/digitalocean/doctl/releases/download/v${DOCTL_VERSION}/doctl-${DOCTL_VERSION}-linux-${DOCTL_ARCH}.tar.gz" && \
    tar xf "doctl-${DOCTL_VERSION}-linux-${DOCTL_ARCH}.tar.gz" && \
    mv doctl /usr/local/bin && \
    rm "doctl-${DOCTL_VERSION}-linux-${DOCTL_ARCH}.tar.gz"
# Install gcloud cli
RUN echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] https://packages.cloud.google.com/apt cloud-sdk main" | tee -a /etc/apt/sources.list.d/google-cloud-sdk.list && \
    curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | gpg --dearmor -o /usr/share/keyrings/cloud.google.gpg && \
    apt-get update -y && \
    apt-get install google-cloud-cli -y
# Install azure cli
RUN curl -sL https://aka.ms/InstallAzureCLIDeb | bash

WORKDIR /usr/local/bin
COPY --from=builder /usr/src/app/target/release/stakpak .
RUN chmod +x /usr/local/bin/stakpak

#USER nobody
ENTRYPOINT ["/usr/local/bin/stakpak"]
CMD ["--help"]
