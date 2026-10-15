# 0362 — Les lanceurs démarrent le client Bevy

Date : 2026-10-15. Statut : accepté. Complète les ADR 0117, 0153 et 0159 ; suit l'ADR 0357
(architecture du client Bevy). Numéro 0362 : 0358 est déjà pris par les cuissons du sol.

## Contexte

Le joueur : « il ne faudra pas oublier de mettre à jour le launcher pour Bevy (Windows, Linux et
macOS) ». Les lanceurs (`Lancer Cent Ans.command`, `.sh`, `.bat`, `.exe`) délèguent tous à
`tools/launch.sh`, qui compilait le cœur GDExtension, importait les ressources Godot et lançait
Godot. Le client Bevy (`client/`, workspace Cargo séparé) est devenu le client à livrer.

## Décision

1. **`tools/launch.sh` démarre Bevy par défaut.** Étapes : mise à jour `stable` (inchangée,
   ADR 0159) ; `cargo build --release --manifest-path client/Cargo.toml` (cargo décide de ce qui
   est à refaire, comme `core/build.sh` pour l'extension) ; paquets de modèles générés et de
   relief fin (inchangés, communs aux deux clients) ; exécution de
   `client/target/release/cent-ans[.exe]` avec `CENT_ANS_ROOT` = racine du dépôt. Les arguments
   après `--` vont au client. Le seul lancement qui prend le focus reste celui du joueur : ce
   script n'est jamais lancé par un agent avec fenêtre.
2. **`--godot`** garde l'ancien chemin intact (recherche de Godot, GDExtension, import headless,
   `-- <arguments Godot>`) pendant la transition. À supprimer avec `game/` quand Bevy a pris la
   relève.
3. **`--dry-run`** affiche le plan (cargo trouvé ou non, bibliothèques manquantes, commande de
   compilation, paquets, ligne de lancement) sans mise à jour, compilation ni lancement. C'est le
   chemin de test des agents et de la CI.
4. **Linux** : avant de compiler, `check_linux_libraries` vérifie `pkg-config`, `alsa`,
   `libudev` et un chargeur Vulkan, et nomme ce qui manque avec la commande d'installation
   Debian/Ubuntu, Fedora et Arch. Échec clair plutôt qu'une erreur de l'éditeur de liens.
5. **Windows** : `Lancer Cent Ans.exe` ne change pas (il lance `tools/launch.sh` dans Git Bash,
   ADR 0153), donc pas de recompilation croisée du binaire commité. Le script cherche `cargo`
   dans `~/.cargo/bin` et `%USERPROFILE%\.cargo\bin`, et dit d'installer rustup et les Build Tools
   Visual Studio (C++) s'il manque. Godot n'est plus un prérequis hors `--godot`.
6. **Sans cargo** : si un binaire déjà compilé existe, il est lancé tel quel (comme la
   bibliothèque du cœur avant) ; sinon message vers rustup.
7. **CI** (`windows.yml`) : l'étape Godot passe `--godot` (le lanceur testait Godot). Nouveau job
   `bevy` sur ubuntu, macOS et Windows : `--dry-run`, `cargo build --release` (exactement la
   commande du lanceur), `cargo test --release --lib`, et sous Linux un essai de fenêtre
   (Vulkan logiciel + xvfb, `continue-on-error`). `promote` attend `smoke` **et** `bevy` : un
   client qui ne compile pas n'atteint pas `stable`. Les tests `tools/tests/test_launch_update.py`
   passent `--godot` et gagnent deux cas (moteur par défaut, `--dry-run`).

## Conséquences

- Le premier lancement compile Bevy en release : 10 à 30 minutes, dit avant de commencer. Les
  suivants sont incrémentaux.
- Les mises à jour de `stable` déclenchent une recompilation du client à chaque lancement qui
  suit une mise à jour (cargo ne refait que les crates touchées).
- Le job `bevy` allonge la promotion de `stable` du temps de la plus lente des trois compilations ;
  le cache `rust-cache` limite le coût après la première fois.
- Non vérifié : l'essai de fenêtre Linux en CI (pilote logiciel), la compilation sous Windows et
  Linux (aucune machine locale), la détection des bibliothèques sur d'autres distributions.
- Pas de fenêtre au premier plan sous macOS : un binaire nu lancé depuis le Terminal ou le
  `.command` du joueur suffit ; l'enveloppe `.app` de `client/run_bg.sh` reste réservée aux agents.
