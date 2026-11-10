# Architettura

## Stack

| Parte | Scelta |
|---|---|
| Linguaggio | Rust |
| Grafica e GPU | wgpu, shader WGSL |
| Finestra e input | winit tramite egui-winit |
| Interfaccia | egui |
| Immagini di verifica | png |
| In seguito | glam, rayon, cpal + fundsp, serde + RON, fluent, fitsrs, cdshealpix, csv, argmin, rfd |

## Struttura

- `crates/fisica`: libreria senza grafica. Ogni formula è testata contro valori
  noti. È ciò che autorizza il bollino "fisica reale".
  `geodetica.rs` è il motore del banco: moto di luce e materia, con Einstein o Newton.
- `crates/app`: finestra, interfaccia, rendering e shader. `lab.rs` contiene lo
  stato del banco e gli esperimenti, senza interfaccia; `banco.rs`, `strumenti.rs`
  e `pannello.rs` la disegnano.

## Il laboratorio

Lo schermo è un banco di prova, non un pannello di informazioni:

- **Banco**: mappa del piano equatoriale. Si lancia trascinando, l'esito è previsto
  mentre si trascina, ogni lancio ha maniglie modificabili.
- **Telescopio**: come apparirebbe la stessa scena. Le scie sono scritte in una
  mappa log-polare del piano, letta dallo shader quando un raggio attraversa il
  piano: per questo si vedono deviate. Gli oggetti sono al più 16 punti luminosi.
- **Strumenti**: condizioni iniziali numeriche, letture, grafici, registro, CSV.
- **Tempo condiviso**: comanda l'orologio lontano (nessuno attraversa l'orizzonte)
  oppure quello di ciascun oggetto (si arriva al centro).

## Regole

- La fisica usa `f64`. Gli shader sono a 32 bit e servono solo a disegnare.
- Nelle scene del buco nero l'unità di lunghezza è il raggio di Schwarzschild.
  L'immagine non dipende dalla massa: cambiano i numeri reali e i confronti.
- I sistemi caotici (tre corpi) girano su CPU a 64 bit, mai in GPU.
- Lo schema di integrazione dei raggi è identico in `fisica::luce` e in
  `crates/app/src/nero.wgsl`. I test del primo validano il secondo.
- Il programma può scattare un'immagine senza finestra (`--scatta`), per
  verificare il rendering in automatico.

## I banchi di relatività

Ogni banco ha stato e interfaccia nel proprio file (`gemelli.rs`, `cadere.rs`,
`curva.rs`); la fisica sta in `fisica::relativita`. Senza telescopio il buco nero
non viene calcolato.

- **Gemelli**: spazio-tempo piatto, linea d'universo modificabile, cambio di osservatore.
- **Perché cadiamo**: campo debole, tempo proprio al primo ordine in `g h / c²` e `v² / c²`.
- **Cosa curva**: palla di particelle cadute da ferme; il volume misura `-4π G ρ`.

Le interazioni si provano senza finestra, dando eventi di input a egui
(`pannello.rs`, modulo `prove`).

## Banchi degli oggetti reali

- **Neutroni** (`neutroni.rs`, `fisica::stelle`): equazioni di stato a tratti di
  Read et al. 2009, equilibrio relativistico integrato nell'entalpia. I test
  controllano le masse massime pubblicate. Misure reali: NICER.
- **Giganti** (`giganti.rs`, `fisica::crescita`): crescita al limite di Eddington
  nel tempo cosmico (Planck 2018), confrontata con quasar misurati.

## Osservatorio

`osservatorio.rs` e `fisica::osservatorio`: orbite di Keplero adattate a misure
reali (Levenberg-Marquardt), con incertezze dalla matrice di covarianza. Si regola
prima a mano, poi si calcola. Le misure si tolgono col clic. Il CSV importato deve
avere l'intestazione esatta: unità e incertezze sono obbligatorie.

## Tre corpi

`trecorpi.rs` e `fisica::trecorpi`: Dormand-Prince a passo adattivo su `f64`, nel
piano. La galleria si valida da sola (le orbite periodiche devono richiudersi).
La mappa del caos si calcola in sottofondo su tutti i processori meno uno, dal
grossolano al fine, e solo quando la pagina viene aperta.

## Galassie, onde, fondo cosmico

- **Galassie** (`galassie.rs`): metodo ristretto di Toomre, galassie come potenziali e
  stelle come particelle di prova. L'attrito dinamico è approssimato.
- **Onde** (`onde.rs`): spirale al quadrupolo, raccordo fenomenologico per fusione e
  assestamento. Il suono usa `cpal` (`audio.rs`).
- **Fondo cosmico** (`fondo.rs`): età, ionizzazione di Saha, scala acustica calcolate.
  Le altezze dei picchi sono indicative. Il cielo è simulato dallo spettro: non è la
  mappa reale, ed è scritto sullo schermo.

## Scala, visite, browser

- **Scala** (`scala.rs`): oggetti reali come cerchi concentrici, zoom logaritmico.
- **Visite** (`visite.rs`): ogni tappa è un testo e una funzione che prepara i banchi.
- **Browser**: stesso codice compilato in WebAssembly, con WebGPU. Senza thread: mappa
  del caos e cielo si calcolano a piccole dosi nei fotogrammi, e la mappa si ferma a
  una risoluzione più bassa. Il salvataggio su file non è disponibile. Costruzione con
  `web/costruisci.ps1`; la versione di `wasm-bindgen` deve coincidere con `Cargo.lock`.

## Dati pubblici

In `crates/app/dati`, ricavati da file pubblici e compilati dentro il programma:
WMAP ILC 9 anni (NASA, ridotta a 720x360), GW150914 H1 (LIGO), Planck 2018 TT,
Pantheon+, Gaia DR3 (4000 stelle entro 50 pc), SPARC (6 galassie). Le orbite delle
stelle centrali vengono da Gillessen et al. 2017. Le missioni condividono un solo
adattamento in `fisica::missioni`.

## Modello di spazio-tempo

Oggi: buco nero non rotante. Previsti: piatto, campo debole, rotante, universo
in espansione, dietro un'unica interfaccia usata da tutte le scene.

## Roadmap

1. **M1 Buco nero** (fatto): banco di prova con luce, sonde, orologi, misure; telescopio.
2. **M2 Relatività sul banco** (fatto): gemelli, perché cadiamo, cosa curva lo spazio-tempo.
3. **M3 Stelle di neutroni e supermassicci** (fatto): costruire una stella, farla crollare, far crescere un gigante.
4. **M4 Osservatorio** (fatto): S2, stelle centrali, LIGO, pulsar binaria, galassie, supernovae, Gaia, Planck.
5. **M5 Tre corpi** (fatto): galleria, caos, mappa, Lagrange, finali, Sistema Solare, pianeta.
6. **M6 Galassie, onde gravitazionali, radiazione di fondo** (fatto), con mappa reale e vita delle stelle.
7. **M7 Zoom globale, visite guidate, versione browser** (fatto).
