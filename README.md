# Colossi

Laboratorio di fisica per far percepire i grandi oggetti dell'universo a chi non
è del mestiere. Rust e wgpu. Stato: quattordici banchi di prova, selezionabili in alto
(scala, buco nero, gemelli, perché cadiamo, cosa curva, stelle di neutroni, giganti,
osservatorio sui dati reali, tre corpi, galassie, onde, fondo cosmico, stelle, orizzonte).

Si prova nel browser, senza scaricare nulla (serve WebGPU):
https://riccardogenco.github.io/ExploringAstrophysic/

```
cargo run -p colossi
cargo test --workspace
```

Sul banco del buco nero: `1` Misura, `2` Luce, `3` Sonda, `4` Fascio, `5` Orologio. Si lancia
trascinando, Maiusc aggancia l'orbita circolare, rotella per lo zoom, tasto destro
per spostare la mappa. `Spazio` pausa, `.` un passo, `R` ricomincia, `Canc` elimina.
I dati esportati finiscono in `misure/`.

Versione browser (serve WebGPU): lancia `web/costruisci.ps1`, poi servi la cartella
`web` con un server locale, per esempio `python -m http.server 8123 --directory web`.

Immagine senza finestra, per le verifiche:

```
cargo run -p colossi -- --scatta prova.png --esperimento 0 --avanza 9000
```

Documenti in `docs/`: `VISIONE.md`, `ARCHITETTURA.md`, `DECISIONI.md`.
