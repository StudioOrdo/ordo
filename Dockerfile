FROM node:22-bookworm-slim AS next-builder
WORKDIR /app
ENV NEXT_TELEMETRY_DISABLED=1

COPY package.json package-lock.json ./
RUN npm ci

COPY app ./app
COPY components ./components
COPY lib ./lib
COPY public ./public
COPY next.config.ts next-env.d.ts tsconfig.json ./
RUN npm run build

FROM rust:1-bookworm AS rust-builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release -p ordo-daemon

FROM node:22-bookworm-slim AS runtime
WORKDIR /app

ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    ORDO_DB_PATH=/app/.data/local.db \
    ORDO_DAEMON_URL=http://127.0.0.1:17760 \
    NEXT_PUBLIC_ORDO_DAEMON_WS_URL=ws://127.0.0.1:17760/ws \
    HOSTNAME=0.0.0.0 \
    PORT=3000

RUN mkdir -p /app/.data

COPY --from=rust-builder /app/target/release/ordo-daemon /usr/local/bin/ordo-daemon
COPY --from=next-builder /app/.next/standalone ./next
COPY --from=next-builder /app/.next/static ./next/.next/static
COPY --from=next-builder /app/public ./next/public
COPY docker/ordo-next /usr/local/bin/ordo-next
RUN chmod +x /usr/local/bin/ordo-next

EXPOSE 3000 17760
VOLUME ["/app/.data"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
  CMD node -e "async function check(url){const response=await fetch(url);if(!response.ok)throw new Error(url+' '+response.status)}Promise.all([check('http://127.0.0.1:17760/ready'),check('http://127.0.0.1:3000/')]).catch((error)=>{console.error(error.message);process.exit(1)})"

CMD ["ordo-daemon", "serve", "--host", "0.0.0.0", "--port", "17760", "--db-path", "/app/.data/local.db", "--next-command", "/usr/local/bin/ordo-next"]