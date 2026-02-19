FROM node:22-alpine AS web-build
WORKDIR /app/apps/web
COPY apps/web/package*.json ./
RUN npm ci
COPY apps/web .
RUN npm run build

FROM rust:1.88 AS api-build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY apps/api apps/api
RUN cargo build --release -p envelopezero-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=api-build /app/target/release/envelopezero-api /usr/local/bin/envelopezero-api
COPY --from=api-build /app/apps/api/migrations /app/apps/api/migrations
COPY --from=web-build /app/apps/web/dist /app/apps/web/dist
ENV PORT=8080 WEB_DIST_DIR=/app/apps/web/dist
EXPOSE 8080
CMD ["envelopezero-api"]
