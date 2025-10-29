# ./Dockerfile
FROM lukemathwalker/cargo-chef:latest-rust-1.89.0 AS chef
WORKDIR /bot

FROM chef AS planner
COPY . .
# compute a lock-like file
RUN cargo chef prepare --recipe-path recipe.json

# use the latest Rust stable release as the base image
FROM chef AS builder
COPY --from=planner /bot/recipe.json recipe.json
# build our project dependencies, not our application
RUN cargo chef cook --release --recipe-path recipe.json
# up to this point, if the dependency tree is the same, all layers should be cached

# copy all files from working environment to our docker image
COPY . .
# explicitly copy over .sqlx querys
COPY ./.sqlx ./.sqlx

# force usage of prepared .sqlx queries when building
ENV SQLX_OFFLINE=true

# build the project
RUN cargo build --release --bin hangoutinator

FROM debian:bookworm-slim AS runtime
WORKDIR /bot
# install OpenSSL and ca-cerificates
RUN apt-get update -y \
	&& apt-get install -y --no-install-recommends openssl ca-certificates \
	# clean up
	&& apt-get autoremove -y \
	&& apt-get clean -y \
	&& rm -rf /var/lib/apt/lists/*

# copy the compiled binary from the builder environment to our runtime environment
COPY --from=builder /bot/target/release/hangoutinator hangoutinator

# when `docker run` is executed, launch the binary
ENTRYPOINT [ "./hangoutinator" ]
