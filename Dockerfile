# syntax=docker/dockerfile:1

# ---------- Frontend build ----------
FROM node:22-alpine AS frontend
WORKDIR /app

COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

# ---------- Backend build ----------
FROM rust:1-bookworm AS backend
WORKDIR /app

# Compile dependencies first so source-only changes are fast rebuilds.
COPY backend/Cargo.toml backend/Cargo.lock* ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY backend/src ./src
COPY backend/migrations ./migrations
RUN touch src/main.rs && cargo build --release

# ---------- Runtime ----------
FROM debian:bookworm-slim

WORKDIR /app

COPY --from=backend /app/target/release/ull-top2000-backend /usr/local/bin/backend
COPY --from=frontend /app/dist ./frontend/dist

ENV APP_ENV=production \
    HOST=0.0.0.0 \
    PORT=8080 \
    FRONTEND_DIR=/app/frontend/dist \
    RUST_LOG=info

# Numeric user: no package installation needed in the runtime image.
USER 10001:10001
EXPOSE 8080

CMD ["backend"]
