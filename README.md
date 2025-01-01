# Applicazione per calcolare i successi in 7th Seas

Backend scritto in Rust che calcola i risultati di dadi e massimizza i successi possibili.  
Progetto ideato per imparare a programmare in Rust e creare una piccola webapp che preveda una parte in Rocket (backend) e una in Yew (frontend).

## Funzionalità
- Generazione casuale dei dadi.
- Calcolo dei Successi e combinazioni valide.
- REST API usando Rocket.
- Interfaccia frontend con Yew per interazione utente.

## Requisiti
- Rust (versione 1.70 o superiore)
- Trunk per il frontend Yew.
- Cargo Make per gestione dei task.

## Come eseguire

### Passaggi Preliminari
1. Clona il repository:
   ```bash
   git clone https://github.com/Makerosophy/seven-seas-successes.git
2. Spostati nella directory del progetto:
   ```bash
   cd seven-seas-successes
3. Installa le dipendenze necessarie:
- Trunk:
   ```bash
   cargo install trunk
   ```
- Cargo make:
   ```bash
   cargo install cargo-make
   ```
- Target WebAssembly per il frontend:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

### Avvio dell'applicazione

Puoi avviare sia il backend che il frontend con uno dei seguenti metodi:

1. Usando Cargo Make
Esegui:
   ```bash
   cargo make start
   ```
Questo comando avvierà il backend e il frontend in parallelo.

2. Usando uno script
   1. Rendi eseguibile lo script start.sh:
   ```bash
   chmod +x start.sh
   ```
   2. Avvia l'applicazione:
   ```bash
   ./start.sh
   ```

### Accesso all'Applicazione
- Frontend: http://localhost:8080
- Backend: http://localhost:8000

## Struttura del Progetto

```bash
project_root/
├── backend/           # Progetto Rocket per il backend
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── frontend/          # Progetto Yew per il frontend
│   ├── Cargo.toml
│   ├── index.html
│   └── src/
│       └── main.rs
├── start.sh           # Script per avviare backend e frontend
├── Makefile.toml      # Configurazione per Cargo Make
└── README.md          # Questo file
```