# The tawks sandbox: devhost in public mode. Build from the repository root:
#   docker build -f tools/devhost/Dockerfile -t tawks-sandbox .
# and run it behind a TLS proxy for your host name:
#   docker run -p 8787:8787 -v sandbox-data:/data -e DEVHOST_PUBLIC_HOST=sandbox.example.com tawks-sandbox
FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p tawks-devhost

FROM debian:bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --uid 10001 sandbox \
  && mkdir /data && chown sandbox /data
COPY --from=build /src/target/release/tawks-devhost /usr/local/bin/tawks-devhost
USER sandbox
ENV DEVHOST_PORT=8787 DEVHOST_DATA_DIR=/data RUST_LOG=info
VOLUME /data
EXPOSE 8787
CMD ["tawks-devhost"]
