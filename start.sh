#!/bin/bash

# Avvia il backend con Cargo
(cd backend && cargo run) &

# Avvia il frontend con Trunk
(cd frontend && trunk serve)