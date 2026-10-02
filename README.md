# Cent Ans

**Jeu de grande stratégie historique sur la guerre de Cent Ans (1337-1453)**, libre et open source.

Une carte de campagne au tour par saison sur l'Europe réelle, puis des batailles et des sièges en 3D en
temps réel avec pause, dans l'esprit de *Total War*. Incarnez la France des Valois, l'Angleterre des
Plantagenêts ou la Bourgogne, et menez un siècle de guerre, de diplomatie, de dynasties et de crises.

![Mêlée de chevaliers autour des étendards de France et d'Angleterre](docs/img/readme/bataille.jpg)

| Paris en 1337 sur la carte de campagne | Londres et la Tamise |
|---|---|
| ![Paris : la Cité, Notre-Dame et la Seine](docs/img/readme/paris.jpg) | ![Londres au bord de la Tamise](docs/img/readme/londres.jpg) |

| Carte de campagne | Bataille de Poitiers (1356) |
|---|---|
| ![Carte de campagne : provinces, armées et relief](docs/img/readme/campagne.jpg) | ![Bataille de Poitiers en temps réel](docs/img/readme/bataille_poitiers.jpg) |

![Écran d'accueil : Notre-Dame au crépuscule](docs/img/readme/menu.jpg)

*Captures en jeu. Pour les refaire : `godot --path game --resolution 1920x1080 --script res://tests/readme_shots.gd -- --out=<dossier>`.*

## Fonctionnalités

- **Carte réelle** de l'Europe occidentale : relief ETOPO/Copernicus, 132 provinces de 1337, fleuves,
  routes, forêts et zones humides de l'époque, déplacement libre des armées.
- **Campagne complète** de 1337 à 1453 (1477 pour la Bourgogne), 29 factions, objectifs historiques.
- **Batailles et sièges 3D** en temps réel avec pause : déploiement, formations, moral, des milliers
  de soldats, relief, engins de siège, murailles qui s'effondrent, incendies ; batailles navales.
- **Villes et économie** : population par classe, bâtiments, commerce, monnaie, impôts, révoltes,
  peste et famine.
- **Personnages et dynasties** : compétences, traits, mariages, successions (loi salique…), régences.
- **Diplomatie et religion** : casus belli, alliances, vassaux, papauté, Grand Schisme, Lollards,
  Hussites.
- **Chronique** : plus d'une centaine d'événements historiques et aléatoires sourcés (Écluse, Peste
  noire, Jacquerie, Jeanne d'Arc…).
- **IA** qui mène une vraie guerre de Cent Ans, avec ses phases et ses alliances historiques.
- **Encyclopédie et codex** : infobulles partout, fiches historiques illustrées.

Interface et textes en français.

## Architecture

```
core/    Rust : toutes les règles du jeu (simulation de campagne, batailles, IA),
         exposées à Godot par une GDExtension (godot-rust)
game/    Godot 4.7 (GDScript) : rendu, interface, entrées — aucune règle de jeu
data/    données de jeu JSON validées par des schémas (data/schemas/), sourcées
tools/   outils Python (uv) : géographie, assets, audio, scripts Blender
docs/    conception, décisions d'architecture (ADR), manuel, historique du projet
```

La simulation est déterministe et testable sans moteur ; Godot ne fait qu'afficher l'état et
transmettre les ordres. Les données (factions, personnages, unités, événements…) vivent dans `data/`,
jamais dans le code.

## Compiler et lancer

Plateformes prises en charge : **macOS Apple Silicon** et **Windows 10/11 x86_64** (ADR 0087), ainsi que
**Linux** (x86_64, arm64) depuis les sources (ADR 0117 ; pas encore d'export ni de CI).

Prérequis : [Rust](https://rustup.rs) stable, [Godot 4.7](https://godotengine.org),
[uv](https://docs.astral.sh/uv/) (outils Python, facultatif pour jouer). Sous Windows, en plus :
[Git pour Windows](https://git-scm.com/download/win) (Git Bash).

**Lancer depuis les sources** : double-cliquer sur le lanceur de son système, à la racine du dépôt.

| Système | Lanceur |
|---|---|
| macOS | `Lancer Cent Ans.command` |
| Linux | `Lancer Cent Ans.sh` |
| Windows | `Lancer Cent Ans.bat` |

Le lanceur (`tools/launch.sh`) recompile le cœur Rust si `core/` a changé. Il refait l'import
headless de Godot si `game/` a changé depuis le dernier import (premier lancement, `git pull`,
autre version de Godot), puis lance le jeu. Il cherche Godot dans la variable `GODOT`, puis dans
le PATH, puis aux emplacements d'installation habituels. Si Godot est ailleurs :
`GODOT=/chemin/vers/godot tools/launch.sh`. Options : `--no-build`, `--import` (import forcé),
et `-- <arguments Godot>`.

À la main, les mêmes étapes sont :

```sh
git clone https://github.com/BenjaminNavet/cent-ans.git
cd cent-ans
core/build.sh                               # compile la GDExtension et la copie dans game/bin/
godot --headless --path game --import       # après chaque changement des ressources
godot --path game                           # lancer le jeu
```

Application autonome : `tools/export_macos.sh` produit `export/Cent Ans.app` (modèles d'export
Godot 4.7 requis).

### Windows

**Jouer** : décompresser `Cent Ans Windows.zip` et lancer `Cent Ans.exe`. Il faut une carte graphique
compatible Vulkan ou Direct3D 12, mais aucune installation supplémentaire (la bibliothèque C de
Microsoft est intégrée à `cent_ans.release.dll`). En cas de problème, `Cent Ans.console.exe` lance le jeu
en gardant son journal affiché.

**Lancer depuis les sources sous Windows** (le dépôt ne contient pas la DLL compilée) :

1. Installer [Git pour Windows](https://git-scm.com/download/win) (fournit Git Bash),
   [Rust](https://rustup.rs) (l'installateur propose les « Visual Studio Build Tools », les accepter)
   et [Godot 4.7.2](https://godotengine.org/download/windows/) (version standard, pas .NET).
2. Dans Git Bash :

   ```sh
   git clone https://github.com/BenjaminNavet/cent-ans.git
   cd cent-ans
   core/build-windows.sh                    # facultatif : le lanceur le fait (quelques minutes)
   ```

3. Double-cliquer sur `Lancer Cent Ans.bat` : il compile `cent_ans.debug.dll`, importe les
   ressources, puis lance le jeu. Si Godot n'est pas trouvé, placer son exécutable
   (`Godot_v4.7.2-stable_win64.exe`) à côté du dossier `cent-ans`, ou définir la variable
   `GODOT`. On peut aussi ouvrir Godot, **Importer** → `cent-ans/game/project.godot`, puis
   **Lancer** (F5).

Le cache du relief fin (`data/map/pyramid/`, ≈ 5 Go) n'est pas dans le dépôt : sans lui, le zoom
rapproché est limité (avis affiché en jeu). Le lanceur le télécharge au premier lancement et à
chaque nouvelle version (il faut [uv](https://docs.astral.sh/uv/) ; `tools/launch.sh --no-relief`
pour s'en passer). À la main : `uv run --project tools cent-ans geo relief-fetch` (voir
`docs/geo.md`).

**Préparer la version Windows depuis un Mac** (compilation croisée avec
[cargo-xwin](https://github.com/rust-cross/cargo-xwin), détails dans `docs/tools.md`) :

```sh
rustup target add x86_64-pc-windows-msvc
brew install llvm lld && cargo install --locked cargo-xwin
tools/export_windows.sh                     # export/windows/ et export/Cent Ans Windows.zip
```

Le workflow GitHub Actions `windows` compile la DLL et lance le test de fumée sur une vraie machine
Windows (`gh workflow run windows.yml`).

## Tests

```sh
cd core && cargo test                                          # simulation Rust
uv run --project tools pytest                                  # outils Python
godot --headless --path game --script res://tests/smoke.gd    # smoke test Godot
```

## Documentation

- Manuel du joueur : [`docs/manuel.md`](docs/manuel.md)
- Conception : [`docs/design/2026-09-23-cent-ans-design.md`](docs/design/2026-09-23-cent-ans-design.md)
- Décisions d'architecture : [`docs/decisions/`](docs/decisions/)
- Données géographiques et leur traitement : [`docs/geo.md`](docs/geo.md)
- État du projet et feuille de route : [`docs/status.md`](docs/status.md), [`docs/roadmap.md`](docs/roadmap.md)

## Contribuer

Les contributions sont bienvenues : issues, corrections historiques, équilibrage, portages.
Conventions : code, identifiants et commits en anglais ; documentation et interface en français.
Avant un commit Rust : `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`. Toute donnée
historique ajoutée cite ses sources (champ `sources`).

Le projet est développé avec l'assistance de Claude Code (Anthropic) ; `CLAUDE.md` et `docs/wip/`
contiennent les consignes et notes de travail des agents.

## Remerciements

Cent Ans n'existerait pas sans les jeux qui l'ont inspiré. Merci à leurs équipes pour leur travail :

- **[Total War](https://www.totalwar.com/)** (Creative Assembly) : l'alliance d'une carte de campagne et
  de batailles en temps réel où l'on voit chaque régiment se battre.
- **[Crusader Kings](https://www.paradoxinteractive.com/games/crusader-kings-iii/about)** (Paradox
  Interactive) : les dynasties, les personnages et les intrigues qui donnent vie au Moyen Âge.


## Licence

- **Code** (`core/`, `game/` hors assets, `tools/`) : [GNU GPL v3.0](LICENSE).
- **Assets et données originaux** du projet : [CC BY-SA 4.0](LICENSE-ASSETS.md).
- **Assets tiers** (`game/assets/third_party/`, icônes, données géographiques) : leurs licences
  propres (CC0, CC BY, OFL, domaine public…), détaillées dans [`CREDITS.md`](CREDITS.md).

*Total War* est une marque de Creative Assembly / SEGA, *Crusader Kings* une marque de Paradox
Interactive, *Civilization* une marque de Take-Two Interactive. Ce projet n'est affilié à aucun de
ces éditeurs ni approuvé par eux ; il n'utilise aucun de leurs assets.
