# Build stage
FROM rust:1.97-alpine AS builder

RUN apk add --no-cache alpine-sdk cmake git ca-certificates

WORKDIR /cuprate
COPY . .

ARG FEATURES="jemalloc"

# Persist the registry and target file across builds. See: https://docs.docker.com/build/cache/optimize/#use-cache-mounts
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/cuprate/target \
    cargo build --release --locked --bin cuprated --features "$FEATURES" \
    && cp target/release/cuprated /usr/local/bin/cuprated

RUN mkdir -p /skel/.local/share/cuprate \
             /skel/.config/cuprate \
             /skel/.cache/cuprate \
    && echo 'cuprate:x:1000:1000::/home/cuprate:/sbin/nologin' > /passwd \
    && echo 'cuprate:x:1000:' > /group

# Runtime stage
FROM scratch

LABEL org.opencontainers.image.source="https://github.com/Cuprate/cuprate"

COPY --from=builder /passwd /etc/passwd
COPY --from=builder /group  /etc/group
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /usr/local/bin/cuprated /usr/local/bin/cuprated
COPY --from=builder --chown=1000:1000 /skel /home/cuprate

USER 1000:1000
WORKDIR /home/cuprate
ENV HOME=/home/cuprate

# P2P ports
EXPOSE 18080/tcp 28080/tcp 38080/tcp
# Restricted RPC ports
EXPOSE 18089/tcp 28089/tcp 38089/tcp

ENTRYPOINT ["/usr/local/bin/cuprated"]
