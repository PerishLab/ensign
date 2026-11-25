FROM docker.io/library/node:24-bookworm-slim
RUN useradd --system --uid 10002 --home /web web
WORKDIR /web
COPY dist ./dist
USER web
ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080
CMD ["node", "dist/.perish/server.mjs", "dist"]
