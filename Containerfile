FROM docker.io/library/debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd --system --uid 10001 --home /ensign ensign
WORKDIR /ensign
COPY ensign-api /usr/local/bin/ensign-api
RUN chmod 0755 /usr/local/bin/ensign-api && mkdir -p .local && chown -R ensign:ensign /ensign
USER ensign
EXPOSE 3500
ENTRYPOINT ["ensign-api"]
CMD ["serve", "/ensign"]
