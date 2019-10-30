FROM rust:1.38 as build

RUN mkdir /project \
 && cd /project \
 && USER=root cargo new --lib temperature-app \
 && USER=root cargo new --bin graphql-server \
 && USER=root cargo new --bin dummy-data-loader

WORKDIR /project

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./temperature-app/Cargo.toml ./temperature-app/Cargo.toml
COPY ./graphql-server/Cargo.toml ./graphql-server/Cargo.toml
COPY ./dummy-data-loader/Cargo.toml ./dummy-data-loader/Cargo.toml

RUN cargo build --release
RUN rm ./temperature-app/src/*.rs \
 && rm ./graphql-server/src/*.rs \
 && rm ./dummy-data-loader/src/*.rs

COPY ./temperature-app/src ./temperature-app/src
COPY ./graphql-server/src ./graphql-server/src
COPY ./dummy-data-loader/src ./dummy-data-loader/src

RUN find target \( -name '*graphql-server*' -o -name '*temperature-app*' -o -name '*dummy-data-loader*' \) -print -exec rm -rf {} +
RUN cargo build --release

FROM rust:1.38
COPY --from=build /project/target/release/graphql-server /usr/local/bin/graphql-server
COPY --from=build /project/target/release/dummy-data-loader /usr/local/bin/dummy-data-loader
CMD ["/usr/local/bin/graphql-server"]
