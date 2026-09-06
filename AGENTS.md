# Kiwi — Robotique, firmware MCU embarqué

## Architecture

- **crates/kiwi-mcu/** — firmware embarqué (seul code source) pour STM32F767ZI (NUCLEO 767ZI).
- **CAO/** — fichiers SolidWorks (gitignorés), pas de build associé.
- Pas de workspace Cargo global : tout se fait dans `crates/kiwi-mcu/`.

## Commandes

```sh
# Build (dans crates/kiwi-mcu/)
cargo build --release

# Flash + run (utilise le runner configuré dans .cargo/config.toml → remote-run.sh)
cargo run --release

# Flash only (via probe-rs sur le Raspberry Pi)
./remote-flash.sh target/thumbv7em-none-eabihf/release/kiwi-mcu

# Debug (lance probe-rs gdb server sur le Pi, port 3333)
./remote-debug.sh target/thumbv7em-none-eabihf/release/kiwi-mcu
```

La cible est `thumbv7em-none-eabihf`. Les scripts `remote-*.sh` utilisent `probe-rs` installé sur le Raspberry Pi à
`192.168.1.133`.

## Débogage et logging

- Logging via `defmt` + RTT. Niveau par défaut : `DEFMT_LOG=info` (défini dans `.cargo/config.toml`).
- Pour augmenter la verbosité : `DEFMT_LOG=trace cargo run --release`.
- `.gdbinit` : désactive la pagination, breakpoint sur `main`, puis continue.

## Matériel, pin mapping

- **TIM3 CH1 (PA6)** → PWM moteur 1 (BLM3650 ESC), 20 kHz edge-aligned
- **TIM3 CH2 (PA7)** → PWM moteur 2
- **TIM3 CH3 (PB0)** → PWM moteur 3

Les ESCs BLM3650 ont un réducteur planétaire 1:36 intégré.

## Stack logicielle

- `no_std` + `embassy` (async executor thread-mode)
- `defmt` + `defmt-rtt` pour logging
- `panic-probe` pour les panics
- `cortex-m` / `cortex-m-rt` runtime
- Pas de tests, pas de CI.

## Inventaire complet du matériel

Voir `materiel.md` à la racine : composants électroniques, moteurs, batteries, lidars, outillage.