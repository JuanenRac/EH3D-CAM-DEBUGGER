<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-CAM-DEBUGGER banner" width="100%">
</p>

# 📖 EH3D-CAM-DEBUGGER

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | 🇮🇹 <b>Italiano</b> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### L'ultimo sguardo a un programma G-code prima che arrivi a una macchina: tempo e avvisi

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.3-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verifica di onestà - cosa funziona oggi:** una libreria Rust e un programma a riga di comando (`eh3d-camdebug`) con 48 test eseguiti su un computer: calcola il tempo di un programma G-code e trova movimenti rapidi nel materiale, movimenti fuori dal piano, tagli a mandrino fermo e teste sopra i morsetti. Non ha ancora una finestra, non è stato provato su una macchina e non è stato verificato su una raccolta di file reali.

---

> **Stato di sicurezza:** questo programma non controlla alcuna macchina e non è un sistema di sicurezza. Ciò che produce deve essere controllato da una persona prima di eseguire qualsiasi cosa su una macchina; nulla di questo ha girato su hardware reale.


## 📋 In breve

| | |
|---|---|
| Versione | `0.0.3` |
| Maturità | Impalcatura |
| Test | 48 |
| Stack tecnologico | Rust |
| Su hardware reale | Non provato: nulla di questo ha girato su una macchina |

**Vai a:** [Panoramica](#-panoramica) · [Architettura](#-architettura) · [Avvio rapido](#-avvio-rapido) · [Test](#-test) · [Struttura](#-struttura-del-repository) · [Correlati](#-progetti-correlati)

## 🚦 Stato del progetto

Ogni riga è una parte di questo repository e il semaforo dice a che punto è oggi. È scritta in base a ciò che il repository può mostrare (test, compilazioni, esecuzioni), non a ciò che è previsto.

🟢 Fatto e verificato · 🟡 Funziona, con limiti noti · 🟠 Fatto a metà o non ancora verificato · 🔴 Non fatto o non funzionante

| Area | Stato | Cosa significa qui |
|---|---|---|
| Nucleo | 🟡 | calcola il tempo di un programma G-code e trova movimenti rapidi nel materiale, movimenti fuori dal piano, tagli a mandrino fermo e teste sopra i morsetti; provato su un computer con input normali e danneggiati, non con una raccolta di file reali. |
| Riga di comando | 🟡 | `eh3d-camdebug` esegue le stesse funzioni della libreria e stampa il risultato; ogni errore torna come messaggio e codice di uscita diverso da zero. |
| Test | 🟡 | 48 test passano (`cargo test`), con casi normali, casi limite e input danneggiati. |
| Sicurezza | 🟡 | Rust sicuro e semplice (`forbid(unsafe_code)`): nulla va in panico con i file ricevuti e un valore errato è un errore con il suo motivo; nessuna revisione indipendente. |
| Finestra | 🔴 | Previsto: la finestra egui comune alla suite; oggi esiste solo la riga di comando. |
| Hardware reale | 🔴 | Non provato: nulla di questo è stato eseguito su una macchina. |
| Documentazione | 🟡 | README in otto lingue, il changelog e le due immagini disegnate da `docs/`. |

## 🎯 Panoramica

* **Il tempo** (`src/timeline.rs`): ogni movimento è un trapezio di velocità da fermo a fermo con l'accelerazione e la velocità rapida della macchina; gli archi si misurano lungo l'arco, un taglio senza avanzamento ne usa uno predefinito e il totale si divide in taglio e rapido. È un limite superiore: un controllore reale non si ferma tra i movimenti.
* **Gli avvisi** (`src/checks.rs`): un movimento rapido (G0) nel materiale o di lato sotto la superficie, un movimento fuori dal piano, una Z sotto la più bassa della macchina e una testa che passa sopra un morsetto sotto la sua altezza sono allarmi (codice di uscita 1); un taglio a mandrino fermo, un taglio senza avanzamento e un avanzamento sopra un limite sono avvertenze. Un movimento rettilineo viene verificato esattamente contro ogni staffa, quindi si trova anche una staffa più sottile di qualsiasi passo.
* **Ciò che il lettore non ha capito** (compensazione del raggio, cicli fissi, una riga danneggiata) appare come allarme con la sua riga, così un programma letto male non è mai dato per pulito.
* **Archi, `--strict` e codici che spostano l'origine:** i limiti del piano e i morsetti sono verificati lungo l'arco di un G2 o G3, non lungo la sua corda; `--strict` trasforma ogni avviso in allarme (codice di uscita 1), per un flusso che usa il codice di uscita come barriera prima della macchina; un programma con G92, G53, G28, G30 o G10 viene segnalato invece di passare per pulito.
* **Lettura e avvisi per codice:** il programma viene letto dai suoi byte (`--max-bytes N` rifiuta un file oltre il limite, 256 MiB di default; il testo non UTF-8 viene letto come Latin-1 e segnalato con il codice `input-not-utf8`); ogni rilievo del lettore mostra il suo codice stabile, e `--ignore CODE[,CODE]` omette gli avvisi di quei codici.

## 🧩 Ruolo in EH3D

EH3D-CAM-DEBUGGER è un programma della EH3D Fabrication Suite. Oggi funziona come libreria e riga di comando; la sua finestra, con l'aspetto comune agli altri programmi della suite, è prevista. Legge e scrive file semplici, quindi può essere usato da solo o tra gli altri programmi.

## 🏛️ Architettura

<p align="center">
  <img src="images/architecture.svg" alt="Architettura di EH3D-CAM-DEBUGGER: chi lo usa, che cosa contiene e da che cosa dipende" width="100%">
</p>


Come si incastrano le parti di questo repository: chi lo usa, cosa contiene e da cosa dipende. L'immagine è disegnata da `docs/architecture.json` e verificata con il codice.

## ✨ Funzionalità

* **Libreria e riga di comando:** ogni comando di `eh3d-camdebug` è una funzione della libreria, così un altro programma della suite può chiamarla senza avviare un processo.
* **Test con input danneggiati:** i test forniscono anche file vuoti, spazzatura, numeri enormi e file tagliati, e nessuno può andare in panico.
* **Nessuna ipotesi:** un valore privo di senso è un errore che dice quale e perché, mai un risultato che sembra solo giusto.
* **Nessun codice unsafe:** `#![forbid(unsafe_code)]` nella libreria e nel programma.

## 🔒 Sicurezza e protezione

| Minaccia | Difesa |
|---|---|
| Un file danneggiato, tagliato o vuoto | nessun panico: i test tagliano un input valido in ogni posizione e forniscono spazzatura |
| Un numero privo di senso (negativo, non finito, assurdamente grande) | rifiutato con un errore che nomina il valore; nulla viene arrotondato in qualcosa di plausibile |
| Uso non sicuro della memoria | Rust sicuro e semplice con `forbid(unsafe_code)` |

**Cosa non è difeso:** un risultato prodotto senza errori non è un risultato verificato. Nulla qui conosce i limiti di una macchina e il programma non ha avuto revisione indipendente. Segnalate una vulnerabilità in privato (vedi [SECURITY.md](SECURITY.md)).

## 🔌 Interfacce

La libreria è `eh3d_cam_debugger` (`use eh3d_cam_debugger::...`); la riga di comando è `eh3d-camdebug`. I comandi sono questi; ciascuno stampa il risultato sull'output standard e segnala un errore sull'output di errore con un codice di uscita diverso da zero. Non c'è accesso alla rete.

```text
EH3D-CAM-DEBUGGER/
├── src/
│   ├── checks.rs                                una parte della logica
│   ├── lib.rs                                   la libreria: ciò che usa ogni comando
│   ├── main.rs                                  la riga di comando
│   └── timeline.rs                              una parte della logica
├── tests/
│   ├── debugger.rs                              test della libreria e della riga di comando
│   ├── reading.rs                               test di lettura dei file: il limite di dimensione, testo non UTF-8, avvisi per codice
│   └── readme.rs                                test che eseguono ciò che promettono il README e gli script run
├── examples/
│   └── sample.nc                                un esempio per provare il programma
├── docs/
│   ├── architecture.json                        dati dell'immagine di Architettura
│   └── structure.json                           dati dell'immagine di Struttura
├── tools/                                       controllo della CI e del README e strumenti per le immagini
├── images/                                      banner e le immagini di Architettura e Struttura del repository
├── .github/                                     workflow di CI e di rilascio, modelli di issue e pull request
├── eh3d.project.json                            manifesto: versione, ruolo, maturità e comando di compilazione
├── Cargo.toml Cargo.lock                        il workspace e le sue dipendenze bloccate
├── rust-toolchain.toml                          la toolchain Rust fissata
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

## ⚙️ Configurazione

Non c'è ancora un file di configurazione: ciò che la persona sceglie si passa come argomenti della riga di comando (vedi Interfacce) e ogni valore predefinito è scritto nella guida del comando (`eh3d-camdebug help`). I limiti fissi sono costanti con nome nel codice, coperte da test.

## 🚀 Avvio rapido

Serve una toolchain Rust stabile (1.75 o più recente).

```bash
cargo build --release
cargo test
cargo run --release -- check examples/sample.nc --table 100,80 --clamp 20,-5,30,5,6
```

## 🧪 Test

La suite ha 48 test (`cargo test`, eseguiti su Windows e Linux nella CI). Verificano i casi normali, i casi limite e gli input danneggiati: file vuoti, spazzatura e file tagliati non causano mai un panico. La CI esegue anche `cargo fmt --check`, `cargo clippy` con gli avvisi come errori e `python -I tools/ci_validate.py`.

## 🏗️ Compilazione e rilascio

La versione sta in `Cargo.toml` e in `eh3d.project.json`; ogni versione ha la sua voce in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilano il programma, `test.bat` / `test.sh` eseguono i test e `run.bat` / `run.sh` lo eseguono con un esempio. Le versioni sono pubblicate su GitHub.

## 📂 Struttura del repository

<p align="center">
  <img src="images/repository-structure.svg" alt="Struttura del repository EH3D-CAM-DEBUGGER" width="100%">
</p>

<details>
<summary>Versione testuale dell'albero</summary>

```text
EH3D-CAM-DEBUGGER/
├── crates/
│   ├── eh3d-parser-gerber/                      lettore Gerber RS-274X
│   ├── eh3d-parser-excellon/                    lettore di file di foratura Excellon
│   └── eh3d-parser-gcode/                       lettore e interprete modale di G-code
├── docs/                                        READERS.md, cosa promette ogni lettore e cosa non fa, e i dati delle due immagini
├── tools/                                       controllo della CI e del README e strumenti per le immagini
├── images/                                      banner e le immagini di Architettura e Struttura del repository
├── .github/                                     workflow di CI e di rilascio, modelli di issue e pull request
├── eh3d.project.json                            manifesto: versione, ruolo, maturità e comando di compilazione
├── Cargo.toml Cargo.lock                        il workspace e le sue dipendenze bloccate
├── rust-toolchain.toml                          la toolchain Rust fissata
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

</details>

## 🗺️ Roadmap e limiti noti

**Limiti noti oggi**

* Non ha ancora una finestra: esistono solo la libreria e la riga di comando.
* Non è stato provato su una macchina né con una raccolta di file reali e non ha avuto revisione indipendente.
* Non c'è ancora simulazione 3D né asportazione di materiale, e i morsetti sono scatole descritte a mano.

**Prossimi passi**

* Simulare l'asportazione di materiale in 3D e confrontare il risultato con il pezzo, con la finestra della suite.
* Aggiungere la finestra con l'aspetto comune ai programmi della suite, negli otto idiomi.
* Provarlo con file reali e con gli altri programmi della suite.

## 🌍 Lingue

Il README e i testi esistono in inglese, spagnolo, francese, italiano, tedesco, cinese, giapponese e portoghese europeo.

## 🔗 Progetti correlati

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): i lettori di Gerber, Excellon e G-code
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): la fresatura di isolamento del rame
* [EH3D-ROUTEPRO](https://github.com/JuanenRac/EH3D-ROUTEPRO): il programma di fresatura del contorno
* [EH3D-AUTOLEVEL](https://github.com/JuanenRac/EH3D-AUTOLEVEL): la mappa delle altezze e la correzione del G-code
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): il programma di controllo della macchina

## 📚 Documentazione

* [Changelog](CHANGELOG.md)
* [Dati di architettura e struttura](docs/architecture.json): i dati da cui si disegnano le due immagini.

## 🤝 Contribuire e comunità

* Prima di una pull request eseguite `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tutto deve passare. Vedi [CONTRIBUTING.md](CONTRIBUTING.md) e il [codice di condotta](CODE_OF_CONDUCT.md).
* Il README esiste in otto lingue: una modifica al testo si fa in tutti e otto i file, e le due immagini sono disegnate da `docs/architecture.json` e `docs/structure.json` con `tools/make_architecture.py` e `tools/make_structure.py`.
* Segnalate una vulnerabilità in privato: [SECURITY.md](SECURITY.md). Domande, idee e segnalazioni: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTORE

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENZA

Software: GPL-3.0-or-later. Vedi [LICENSE](LICENSE). I progetti hardware della suite usano la CERN OHL-S.
