#!/bin/bash

# Configura le directory temporanee per Rust
CARGO_HOME=/tmp/.cargo
RUSTUP_HOME=/tmp/.rustup
PATH=$CARGO_HOME/bin:$PATH

# Installa Rust
echo "Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Aggiungi manualmente Cargo al PATH
echo "Adding Cargo to PATH..."
export PATH=$CARGO_HOME/bin:$PATH

# Installa Trunk
echo "Installing Trunk..."
cargo install trunk

# Costruisci il progetto
echo "Building the project..."
trunk build --release --dist frontend/dist