<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-CAM-DEBUGGER banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-CAM-DEBUGGER
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | 🇫🇷 <b>Français</b> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Le dernier regard sur un programme G-code avant qu'il n'atteigne une machine : temps et alertes

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.5-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Vérification d'honnêteté - ce qui fonctionne aujourd'hui :** une bibliothèque Rust et un programme en ligne de commande (`eh3d-camdebug`) avec 113 tests exécutés sur un ordinateur : calcule le temps d'un programme G-code et trouve les mouvements rapides dans la matière, les mouvements hors de la table, les coupes broche arrêtée et les têtes au-dessus de pinces. Il a une fenêtre (compilée avec `--features gui`) qui ouvre un programme G-code, dessine son trajet en 2D, parcourt les mouvements pas à pas avec un tableau des lignes et montre les alertes et avertissements, mais la fenêtre n'a pas encore été vue sur un écran ; n'a pas été essayé sur une machine et n'a pas été vérifié sur une collection de fichiers réels.

---

> **État de sécurité :** ce programme ne commande aucune machine et n'est pas un système de sécurité. Ce qu'il produit doit être vérifié par une personne avant tout essai sur une machine ; rien ici n'a fonctionné sur du matériel réel.


## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.0.5` |
| Maturité | Échafaudage |
| Tests | 113 |
| Pile technique | Rust |
| Sur du matériel réel | Non essayé : rien ici n'a fonctionné sur une machine |

**Aller à  :** [Présentation](#-présentation) · [Architecture](#-architecture) · [Démarrage rapide](#-démarrage-rapide) · [Tests](#-tests) · [Structure](#-structure-du-dépôt) · [Liés](#-projets-liés)

## 🚦 État du projet

Chaque ligne est une partie de ce dépôt et le feu dit où elle en est aujourd'hui. Elle est écrite d'après ce que le dépôt peut montrer (tests, compilations, exécutions), pas d'après ce qui est prévu.

🟢 Fait et vérifié · 🟡 Fonctionne, avec des limites connues · 🟠 À moitié fait ou pas encore vérifié · 🔴 Pas fait ou ne fonctionne pas

| Domaine | État | Ce que cela veut dire ici |
|---|---|---|
| Cœur | 🟡 | calcule le temps d'un programme G-code et trouve les mouvements rapides dans la matière, les mouvements hors de la table, les coupes broche arrêtée et les têtes au-dessus de pinces ; testé sur un ordinateur avec des entrées normales et abîmées, pas avec une collection de fichiers réels. |
| Ligne de commande | 🟡 | `eh3d-camdebug` exécute les mêmes fonctions que la bibliothèque et affiche le résultat ; chaque erreur revient sous forme de message et de code de sortie non nul. |
| Tests | 🟡 | 113 tests passent (`cargo test`), avec des cas normaux, des cas limites et des entrées abîmées. |
| Sécurité | 🟡 | Rust sûr et simple (`forbid(unsafe_code)`) : rien ne panique sur les fichiers reçus et une mauvaise valeur est une erreur avec sa raison ; pas de revue indépendante. |
| Fenêtre | 🟠 | Écrite (egui, `--features gui`) : ouvrir ou déposer un programme, voir son trajet en 2D, parcourir les mouvements pas à pas ou les lire, un tableau des mouvements, les alertes et avertissements, les réglages de la machine et les limites ; sa logique est testée, mais elle n'a pas encore été compilée ni vue sur un écran. |
| Matériel réel | 🔴 | Non essayé : rien ici n'a été exécuté sur une machine. |
| Documentation | 🟡 | README en huit langues, le journal des modifications et les deux images dessinées depuis `docs/`. |

## 🎯 Présentation

* **Le temps** (`src/timeline.rs`) : chaque mouvement est un trapèze de vitesse de l'arrêt à l'arrêt avec l'accélération et la vitesse rapide de la machine ; les arcs sont mesurés le long de l'arc, une coupe sans avance en utilise une par défaut et le total se partage en coupe et rapide. C'est une borne supérieure : un vrai contrôleur ne s'arrête pas entre les mouvements. Les attentes de `G4` (`P` en secondes, en millisecondes dans Marlin avec `S` en secondes) ne sont pas des déplacements et sont ajoutées à part.
* **Les alertes** (`src/checks.rs`) : un mouvement rapide (G0) dans la matière ou de côté sous la surface, un mouvement hors de la table, un Z sous le plus bas de la machine et une tête qui passe au-dessus d'une pince sous sa hauteur sont des alertes (code de sortie 1) ; une coupe broche arrêtée, une coupe sans avance et une avance au-dessus d'une limite sont des avertissements. Un déplacement droit est testé exactement contre chaque bride, de sorte qu'une bride plus fine que n'importe quel pas est quand même trouvée.
* **Ce que le lecteur n'a pas compris** (compensation de rayon, cycles fixes, une ligne abîmée) apparaît comme une alerte avec sa ligne, si bien qu'un programme mal lu n'est jamais déclaré propre.
* **Arcs, `--strict` et codes qui déplacent l'origine :** les limites de la table et les brides sont testées le long de l'arc d'un G2 ou G3, pas le long de sa corde ; `--strict` transforme chaque avertissement en alerte (code de sortie 1), pour un flux qui utilise le code de sortie comme barrière avant la machine ; un programme avec G92, G53, G28, G30 ou G10 est signalé au lieu de passer pour propre.
* **Lecture et avis par code :** le programme est lu depuis ses octets (`--max-bytes N` refuse un fichier au-dessus de la limite, 256 MiB par défaut ; le texte non UTF-8 est lu comme du Latin-1 et signalé avec le code `input-not-utf8`) ; chaque constat du lecteur montre son code stable, et `--ignore CODE[,CODE]` omet les avis de ces codes.

## 🧩 Rôle dans EH3D

EH3D-CAM-DEBUGGER est un programme de l'EH3D Fabrication Suite. Il fonctionne comme bibliothèque et ligne de commande et, compilé avec `--features gui`, comme fenêtre avec l'aspect commun aux autres programmes de la suite. Il lit et écrit des fichiers simples et peut donc s'utiliser seul ou entre les autres programmes.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="Architecture de EH3D-CAM-DEBUGGER : qui l'utilise, ce qu'il contient et de quoi il dépend" width="100%">
</p>


Comment les parties de ce dépôt s'articulent : qui l'utilise, ce qu'il contient et de quoi il dépend. L'image est dessinée depuis `docs/architecture.json` et vérifiée avec le code.

## ✨ Fonctionnalités

* **Une bibliothèque et une ligne de commande :** chaque commande de `eh3d-camdebug` est une fonction de la bibliothèque ; un autre programme de la suite peut l'appeler sans lancer de processus.
* **Tests avec entrées abîmées :** les tests fournissent aussi des fichiers vides, du bruit, des nombres énormes et des fichiers coupés, et aucun ne doit paniquer.
* **Sans deviner :** une valeur absurde est une erreur qui dit laquelle et pourquoi, jamais un résultat qui n'en a que l'air.
* **Pas de code unsafe :** `#![forbid(unsafe_code)]` dans la bibliothèque et le programme.
* **Une avance manquante est signalée une fois, et le mode laser est lu :** chaque déplacement sans avance donne un seul constat (`--ignore feed-missing` le fait taire), et `--laser` lit `M3` ou `M4` avec `S0` comme un laser éteint.

## 🔒 Sécurité et protection

| Menace | Défense |
|---|---|
| Un fichier abîmé, coupé ou vide | pas de panique : les tests coupent une entrée valide à chaque position et fournissent du bruit |
| Un nombre absurde (négatif, non fini, démesuré) | refusé avec une erreur qui nomme la valeur ; rien n'est arrondi en quelque chose de plausible |
| Usage non sûr de la mémoire | Rust sûr et simple avec `forbid(unsafe_code)` |

**Ce qui n'est pas défendu :** un résultat produit sans erreur n'est pas un résultat vérifié. Rien ici ne connaît les limites d'une machine et le programme n'a pas eu de revue indépendante. Signalez une vulnérabilité en privé (voir [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

La bibliothèque est `eh3d_cam_debugger` (`use eh3d_cam_debugger::...`) ; la ligne de commande est `eh3d-camdebug`. Les commandes sont celles-ci ; chacune affiche son résultat sur la sortie standard et signale une erreur sur la sortie d'erreur avec un code de sortie non nul. Il n'y a pas d'accès réseau.

```text
eh3d-camdebug check FILE.nc [options]     the time, the findings; exit code 1 when there is an alert, 2 when there are only warnings, 64 for a wrong option
eh3d-camdebug steps FILE.nc [options]     every move with its line, length and time
eh3d-camdebug [gui] [FILE.nc]             the window (a build with --features gui); with no arguments it opens
options: --accel MM_S2 (500)  --rapid MM_MIN (3000)  --feed MM_MIN (300)  --dialect grbl|marlin|planetcnc  --table WIDTH,DEPTH  --surface Z (0)  --min-z Z  --max-feed MM_MIN
         --laser  --clamp X0,Y0,X1,Y1,HEIGHT  --strict  --ignore CODE[,CODE]  --max-bytes N
```

## ⚙️ Configuration

Il n'y a pas encore de fichier de configuration : ce que la personne choisit est donné en arguments de la ligne de commande (voir Interfaces) et chaque valeur par défaut est écrite dans l'aide de la commande (`eh3d-camdebug help`). Les limites fixes sont des constantes nommées dans le code, couvertes par des tests.

## 🚀 Démarrage rapide

Il faut une chaîne d'outils Rust stable (1.88 ou plus récente).

```bash
cargo build --release
cargo test
cargo run --release -- check examples/sample.nc --table 100,80 --clamp 20,-5,30,5,6
cargo run --release --features gui   # the window
```

## 🧪 Tests

La suite compte 113 tests (`cargo test`, exécutés sous Windows et Linux dans le CI). Ils vérifient les cas normaux, les cas limites et les entrées abîmées : fichiers vides, bruit et fichiers coupés ne font jamais paniquer. Le CI exécute aussi `cargo fmt --check`, `cargo clippy` avec les avertissements comme erreurs et `python -I tools/ci_validate.py`.

## 🏗️ Compilation et publication

La version se trouve dans `Cargo.toml` et `eh3d.project.json` ; chaque version a son entrée dans [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilent le programme, `test.bat` / `test.sh` lancent les tests et `run.bat` / `run.sh` l'exécutent avec un exemple. Les versions sont publiées sur GitHub.

**Téléchargements :** Avec chaque version de ce dépôt sont publiés des programmes prêts à l'emploi pour chaque système : Windows (64 et 32 bits), Linux (64 bits) et macOS (Apple silicon et Intel). Chaque paquet, `cam-debugger-<version>-<système>.zip` ou `.tar.gz`, contient le programme, la licence, ce README, le journal des modifications, l'icône et les fichiers d'exemple, et vient avec son fichier `.sha256` ; la version liste aussi tous les fichiers dans `SHA256SUMS` et `build-manifest.json`. Aucune version n'a encore été publiée.

**Signés et vérifiables :** les paquets d'une version sont signés avec la clé de publication d'EH3D, et le fichier `.sig` de chaque paquet est publié à côté (une signature OpenSSH, espace de noms `eh3d-release`). Vérifiez un téléchargement avant de l'exécuter : comparez son SHA-256 (`sha256sum -c cam-debugger-<version>-<système>.tar.gz.sha256`, ou `Get-FileHash` sous Windows) et vérifiez la signature avec `ssh-keygen -Y verify -f allowed_signers -I eh3d-release -n eh3d-release -s <paquet>.sig < <paquet>`, où `allowed_signers` contient la ligne `eh3d-release <type de clé> <clé publique des versions d'EH3D>`.

## 📂 Structure du dépôt

<p align="center">
  <img src="images/repository-structure.svg" alt="Structure du dépôt EH3D-CAM-DEBUGGER" width="100%">
</p>

<details>
<summary>Version texte de l'arborescence</summary>

```text
EH3D-CAM-DEBUGGER/
├── src/
│   ├── checks.rs                                une partie de la logique
│   ├── lib.rs                                   la bibliothèque : ce que chaque commande utilise
│   ├── main.rs                                  la ligne de commande
│   ├── timeline.rs                              une partie de la logique
│   ├── browse.rs                                la liste des fichiers de la fenêtre d'ouverture d'un fichier
│   ├── gui.rs                                   la fenêtre (egui, avec la fonctionnalité gui) : dessine l'état et transmet ce que fait la personne
│   ├── lang.rs                                  les mots de la fenêtre dans les huit langues
│   ├── viewport.rs                              zoom et déplacement de l'image, en simples nombres
│   └── window.rs                                l'état de la fenêtre sans écran : programme, réglages, mouvements et étapes
├── tests/
│   ├── debugger.rs                              tests de la bibliothèque et de la ligne de commande
│   ├── reading.rs                               tests de lecture des fichiers : la limite de taille, le texte non UTF-8, les avis par code
│   ├── readme.rs                                tests qui exécutent ce que promettent le README et les scripts run
│   └── window_cli.rs                            tests qu'une compilation sans la fonctionnalité gui reste une ligne de commande
├── examples/
│   └── sample.nc                                un exemple pour essayer le programme
├── docs/
│   ├── architecture.json                        données de l'image d'Architecture
│   └── structure.json                           données de l'image de Structure
├── tools/                                       contrôle de la CI et du README et outils d'images
├── images/                                      bannière et images d'Architecture et de Structure du dépôt
├── .github/                                     workflows de CI et de publication, modèles d'issues et de pull requests
├── eh3d.project.json                            manifeste : version, rôle, maturité et commande de compilation
├── Cargo.toml Cargo.lock                        le workspace et ses dépendances verrouillées
├── rust-toolchain.toml                          la toolchain Rust épinglée
├── CHANGELOG.md                                 ce qui a changé dans chaque version
├── SECURITY.md SUPPORT.md                       signaler une vulnérabilité et poser des questions
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   contribuer, code de conduite et licence
```

</details>

## 🗺️ Feuille de route et limites connues

**Limites connues aujourd'hui**

* La fenêtre n'a pas encore été vue sur un écran ; elle montre le trajet vu de dessus (pas de vue 3D) et les textes des constats viennent de la bibliothèque, en anglais.
* Il n'a été essayé ni sur une machine ni avec une collection de fichiers réels, et n'a pas eu de revue indépendante.
* Il n'y a pas encore de simulation 3D ni d'enlèvement de matière, et les pinces sont des boîtes décrites à la main.

**Ensuite**

* Simuler l'enlèvement de matière en 3D et comparer le résultat à la pièce, avec la fenêtre de la suite.
* Voir la fenêtre sur un écran et la terminer.
* L'essayer avec des fichiers réels et avec les autres programmes de la suite.

## 🌍 Langues

Le README et les textes existent en anglais, espagnol, français, italien, allemand, chinois, japonais et portugais européen.

## 🔗 Projets liés

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): les lecteurs de Gerber, Excellon et G-code
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): le fraisage d'isolation du cuivre
* [EH3D-ROUTEPRO](https://github.com/JuanenRac/EH3D-ROUTEPRO): le programme de fraisage de contour
* [EH3D-AUTOLEVEL](https://github.com/JuanenRac/EH3D-AUTOLEVEL): la carte de hauteurs et la correction du G-code
* [EH3D-PHOENIX-CNC](https://github.com/JuanenRac/EH3D-PHOENIX-CNC): le programme de commande de machine

## 📚 Documentation

* [Journal des modifications](CHANGELOG.md)
* [Données d'architecture et de structure](docs/architecture.json) : les données qui servent à dessiner les deux images.

## 🤝 Contribuer et communauté

* Avant une pull request, lancez `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` et `python -I tools/ci_validate.py` : tout doit passer. Voir [CONTRIBUTING.md](CONTRIBUTING.md) et le [code de conduite](CODE_OF_CONDUCT.md).
* Le README existe en huit langues : un changement de texte se fait dans les huit fichiers, et les deux images sont dessinées depuis `docs/architecture.json` et `docs/structure.json` par `tools/make_architecture.py` et `tools/make_structure.py`.
* Signalez une vulnérabilité en privé : [SECURITY.md](SECURITY.md). Questions, idées et rapports : electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTEUR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCE

Logiciel : GPL-3.0-or-later. Voir [LICENSE](LICENSE). Les conceptions matérielles de la suite utilisent la CERN OHL-S.
