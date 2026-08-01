# syntax=docker/dockerfile:1
FROM debian:bookworm-slim AS run
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd --system --uid 10001 --home /ensign ensign
WORKDIR /ensign
COPY deploy/ensign-api /usr/local/bin/ensign-api
COPY deploy/ensign.toml ensign.toml
RUN chmod 0755 /usr/local/bin/ensign-api && mkdir -p .local && chown -R ensign:ensign /ensign
USER ensign
EXPOSE 3500
# The subcommand and its root arrive as args: bootstrap or serve.
ENTRYPOINT ["ensign-api"]
CMD ["serve", "/ensign"]
