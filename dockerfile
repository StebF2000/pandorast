# ----------------------------------------------
#             Cargo build stage
# ----------------------------------------------

FROM rust:latest as cargo-build
WORKDIR /usr/src/pandorast
COPY Cargo.toml Cargo.toml
RUN mkdir src/
RUN cargo build --release


# ----------------------------------------------
#             Second stage
# ----------------------------------------------

# TODO Stuff
