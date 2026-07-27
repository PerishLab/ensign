# syntax=docker/dockerfile:1
FROM node:24-slim AS build
RUN corepack enable
WORKDIR /src
COPY pnpm-workspace.yaml pnpm-lock.yaml package.json ./
COPY apps apps
COPY packages packages
RUN pnpm install --frozen-lockfile && pnpm -r build

FROM nginx:1.27-alpine AS run
ENV API_UPSTREAM=api:3500
ENV NGINX_ENVSUBST_FILTER=API_UPSTREAM
COPY deploy/web.conf.template /etc/nginx/templates/default.conf.template
COPY --from=build /src/apps/web/dist /usr/share/nginx/html
EXPOSE 80
