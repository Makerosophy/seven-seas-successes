#!/bin/bash

# Configura directory temporanee indipendenti da $HOME
CARGO_HOME=/tmp/cargo
RUSTUP_HOME=/tmp/rustup
PATH=$CARGO_HOME/bin:$PATH

# Assicurati che le directory esistano
mkdir -p $CARGO_HOME $RUSTUP_HOME

# Installa Rust
echo "Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path

# Verifica l'installazione di Rust
echo "Verifying Rust installation..."
$CARGO_HOME/bin/rustc --version
$CARGO_HOME/bin/cargo --version

# Installa Trunk
echo "Installing Trunk..."
$CARGO_HOME/bin/cargo install trunk

# Costruisci il progetto
echo "Building the project..."
$CARGO_HOME/bin/trunk build --release --dist frontend/dist