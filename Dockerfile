FROM ubuntu:latest

RUN apt-get update && \
    apt-get install -y curl libssl-dev clang pkg-config && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

RUN curl -OL https://github.com/ethereum/solidity/releases/download/v0.8.19/solc-static-linux && \
    mv solc-static-linux /usr/bin/solc && \
    chmod +x /usr/bin/solc

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup default nightly

WORKDIR /app

COPY . .

RUN cargo build
RUN cd example && solc infinite_mint.sol --base-path . --include-path .. --abi --bin --overwrite -o ./out

