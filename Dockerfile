FROM --platform=$BUILDPLATFORM rust:1.96.1-bookworm AS build

ARG TARGETOS
ARG TARGETARCH

RUN rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu

RUN apt update && apt install -y pkg-config libssl-dev gcc-aarch64-linux-gnu gcc-x86-64-linux-gnu

WORKDIR /build/ddns-runner

COPY . .

RUN if [[ "${TARGETARCH}" == "amd64" ]]; \
    then \
      cargo build --release --target x86_64-unknown-linux-gnu && mv ./target/x86_64-unknown-linux-gnu/release/ddns-runner ./target/release/ddns-runner; \
    else \
      cargo build --release --target aarch64-unknown-linux-gnu && mv ./target/aarch64-unknown-linux-gnu/release/ddns-runner ./target/release/ddns-runner; \
    fi

FROM debian:bookworm-20260623

ARG ADDITIONAL_PACKAGES

RUN apt update && apt install -y libssl-dev ${ADDITIONAL_PACKAGES} && apt clean
# Create a non-privileged user that the app will run under.
# See https://docs.docker.com/go/dockerfile-user-best-practices/
RUN groupadd -r appuser && useradd --no-log-init -r -g appuser -u 10001 --home /nonexistent --shell /sbin/nologin appuser
USER appuser

COPY --from=build /build/ddns-runner/target/release/ddns-runner /ddns-runner/ddns-runner
COPY --chown=appuser:appuser ./migrations /ddns-runner/migrations/
EXPOSE 3000
ENTRYPOINT ["/ddns-runner/ddns-runner"]
