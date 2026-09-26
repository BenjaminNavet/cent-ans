# 0087 — Portage Windows x86_64

Date : 2026-09-26. Statut : accepté.

## Contexte

Le jeu ne tournait que sur macOS Apple Silicon. La logique de jeu est en Rust (`core/`) et
Godot la charge sous forme de bibliothèque native (GDExtension). Cette bibliothèque doit être
compilée pour chaque système : `.dylib` sur macOS, `.dll` sur Windows. Le reste est portable
tel quel : le GDScript, les données (`data/`) et les dépendances Rust (`godot` 0.5, `png`,
`rand_chacha`), qui n'ont aucun code propre à une plateforme.

Le développement se fait sur un Mac. Il n'y a aucune machine Windows sous la main.

## Décision

- **Compilation croisée depuis le Mac** avec `cargo xwin`, pour la cible
  `x86_64-pc-windows-msvc` (`core/build-windows.sh [--release]`). L'outil télécharge le CRT et
  le SDK Windows de Microsoft, puis fait l'édition de liens avec `lld-link` de LLVM. Sous
  Windows (Git Bash, CI), le même script appelle simplement `cargo build`. La cible MSVC est
  celle recommandée par godot-rust et la cible de premier rang de Rust sous Windows.
- **CRT statique** (`core/.cargo/config.toml` : `+crt-static`). La DLL n'importe que des DLL
  système : les joueurs n'ont pas à installer le redistribuable Visual C++.
- **Disposition de l'export Windows** : un dossier plat qui contient `Cent Ans.exe`,
  `Cent Ans.pck` (non intégré), `cent_ans.release.dll`, `data/`, et éventuellement
  `Cent Ans relief/`. `MapPaths` cherche `data/` à côté de l'exécutable après
  l'emplacement macOS (`../Resources/data`). `cent-ans export-data --dir` place les données.
- **Préréglage « Windows Desktop »** avec `application/modify_resources=false`. L'icône et
  les métadonnées de l'exécutable passeraient par rcedit, donc par wine depuis le Mac ; on s'en
  passe pour l'instant. Le wrapper console `Cent Ans.console.exe` est gardé : il affiche le
  journal du jeu pour les rapports de bogue.
- **Vérification sur un vrai Windows** : le workflow GitHub Actions `windows` tourne sur
  `windows-latest`. Il compile la DLL en natif, importe le projet et lance `smoke.gd` en
  headless. Il se déclenche à la main ou sur les branches `windows/**`, car les minutes
  Windows d'un dépôt privé comptent double sur le quota gratuit.

## Conséquences

- `core/build.sh` (macOS) et `core/build-windows.sh` restent séparés. Les deux artefacts
  (`libcent_ans.*.dylib`, `cent_ans.*.dll`) cohabitent dans `game/bin/` et sont ignorés par git.
- Sur le Mac, le `cargo` de Homebrew n'a pas la bibliothèque standard Windows. Le script
  place les proxys rustup (`~/.cargo/bin`) en tête du PATH. Prérequis ponctuels :
  `rustup target add x86_64-pc-windows-msvc`, `brew install llvm lld`,
  `cargo install --locked cargo-xwin`, et les modèles d'export Windows de Godot 4.7.2.
- Premier passage de la CI (2026-09-26, 21 min) : le test de fumée est vert sous Windows, sur
  la vraie simulation Rust (10 tours, commerce, technologies, batailles, siège).
- Pas encore mesuré : le rendu réel (Vulkan / D3D12) et les performances sur un PC. Les
  réglages de performance ont été faits sur un M4. Il faudra une partie d'essai sur un vrai
  PC.
- Seul x86_64 est déclaré, pas Windows ARM64. Linux n'est pas traité, mais suivrait le
  même schéma : cible `x86_64-unknown-linux-gnu` et entrée `linux.*` dans le `.gdextension`.
