#!/bin/bash

# Configura directory temporanee indipendenti da $HOME
export CARGO_HOME=/tmp/cargo
export RUSTUP_HOME=/tmp/rustup
export PATH=$CARGO_HOME/bin:$PATH

# Assicurati che le directory esistano
mkdir -p $CARGO_HOME $RUSTUP_HOME

# Installa Rust
echo "Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path

# Aggiungi Cargo al PATH manualmente
. "$CARGO_HOME/env"

# Verifica l'installazione di Rust e Cargo
echo "Verifying Rust installation..."
rustc --version
cargo --version

# Installa Trunk
echo "Installing Trunk..."
cargo install trunk

# Passa alla directory frontend
cd frontend || exit

# Debug: verifica la configurazione di Trunk
echo "Checking Trunk configuration..."
ls -la Trunk.toml
ls -la index.html

# Costruisci il progetto con Trunk
echo "Building the project..."
trunk build --release --dist dist