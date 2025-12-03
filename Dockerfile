# Multi-stage build for Rustpad with AI, File Freeze, and Admin features

# Build backend
FROM rust:alpine AS backend
WORKDIR /home/rust/src
RUN apk --no-cache add musl-dev openssl-dev openssl-libs-static pkgconfig
COPY rustpad-server ./rustpad-server
WORKDIR /home/rust/src/rustpad-server
ENV OPENSSL_STATIC=1
ENV OPENSSL_LIB_DIR=/usr/lib
ENV OPENSSL_INCLUDE_DIR=/usr/include
RUN cargo build --release

# Build WASM
FROM --platform=amd64 rust:alpine AS wasm
WORKDIR /home/rust/src
RUN apk --no-cache add curl musl-dev
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
COPY rustpad-wasm ./rustpad-wasm
RUN wasm-pack build rustpad-wasm

# Build frontend
FROM --platform=amd64 node:lts-alpine AS frontend
WORKDIR /usr/src/app
COPY package.json package-lock.json ./
COPY --from=wasm /home/rust/src/rustpad-wasm/pkg rustpad-wasm/pkg
RUN npm ci
COPY src ./src
COPY index.html tsconfig.json vite.config.ts ./
COPY public ./public
ARG GITHUB_SHA
ENV VITE_SHA=${GITHUB_SHA}
RUN npm run build

# Final runtime image
FROM alpine:3.19

# Install runtime dependencies
RUN apk --no-cache add ca-certificates libgcc libssl3 libcrypto3

WORKDIR /app

# Copy built artifacts
COPY --from=frontend /usr/src/app/dist ./dist
COPY --from=backend /home/rust/src/rustpad-server/target/release/rustpad-server .

# Create directories for persistent data
RUN mkdir -p /app/frozen_documents/users \
             /app/frozen_documents/frozen \
             /app/artifacts

# Copy .env template (will be overridden by docker-compose)
COPY rustpad-server/.env ./.env.template

# Expose port
EXPOSE 3030

# Set proper permissions
RUN chown -R 1000:1000 /app
USER 1000:1000

CMD [ "./rustpad-server" ]
