FROM docker.io/library/node:24-bookworm-slim AS build
ENV PNPM_HOME=/pnpm
ENV PATH=$PNPM_HOME:$PATH
RUN corepack enable
WORKDIR /srv
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml .npmrc ./
COPY apps/web/package.json apps/web/package.json
RUN pnpm install --frozen-lockfile --filter @perishlab/ensign-web...
COPY apps/web apps/web
RUN pnpm --filter @perishlab/ensign-web build

FROM docker.io/library/node:24-bookworm-slim
RUN useradd --system --uid 10002 --home /web web
WORKDIR /web
COPY --from=build /srv/apps/web/dist ./dist
USER web
ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080
CMD ["node", "dist/.perish/server.mjs", "dist"]
