# syntax=docker/dockerfile:1
FROM node:24-slim AS build
RUN corepack enable
WORKDIR /src
COPY pnpm-workspace.yaml pnpm-lock.yaml package.json ./
COPY apps apps
COPY packages packages
RUN pnpm install --frozen-lockfile && pnpm -r build

FROM node:24-alpine AS run
WORKDIR /app
ENV HOST=0.0.0.0
ENV PORT=8080
COPY --from=build /src/apps/web/dist ./dist
USER node
EXPOSE 8080
CMD ["node", "dist/.perish/server.mjs", "dist"]
