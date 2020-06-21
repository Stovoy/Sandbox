# syntax=docker/dockerfile:experimental
FROM rust:1.44-buster@sha256:b2fd2fcc9d28c1a6dc59c3b0b37913fd9a326c8e457e50617e1156fc1ad51e34 as build
WORKDIR /app
RUN USER=root cargo init

RUN rustup default stable
RUN curl -sL https://deb.nodesource.com/setup_14.x | bash - && \
    apt-get install -y nodejs

COPY . ./
RUN \
 --mount=type=cache,target=/usr/local/cargo/git \
 --mount=type=cache,target=/usr/local/cargo/registry \
 --mount=type=cache,target=/usr/local/cargo/bin \
 --mount=type=cache,target=/app/node_modules \
 --mount=type=cache,target=/app/target \
 --mount=type=cache,target=/app/.cargo \
 --mount=type=cache,target=/root/.cargo \
 npm install && \
 curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh && \
 npm run build && \
 cargo install sfz

FROM debian:buster@sha256:f19be6b8095d6ea46f5345e2651eec4e5ee9e84fc83f3bc3b73587197853dc9e
WORKDIR /app
ENTRYPOINT ["sfz", "-p", "1064"]
COPY --from=build /app/dist .
COPY --from=build /usr/local/cargo/bin/sfz /usr/local/bin/sfz
