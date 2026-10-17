# Cent Ans — instructions projet

Jeu de grande stratégie (guerre de Cent Ans). Lire `docs/design/2026-09-23-cent-ans-design.md` avant tout travail.

## Règles
- Vue d'ensemble du dépôt (crates, `game/`, flux campagne→bataille, ADR clés) : `docs/architecture.md`.
- Code, identifiants, commits en anglais ; docs, UI et communication en français.
- Toute règle de jeu vit dans `core/` (Rust). `client/` (Bevy 0.20, ADR 0357) ne fait que rendu, UI, entrées. `game/assets` est la racine d'assets du client ; `game/resources/*.tres` (réglages lus par Bevy) y reste. Le client Godot a été retiré (ADR 0366).
- Données de jeu dans `data/` (JSON/YAML) validées par `data/schemas/`. Jamais de données codées en dur.
- Rust : `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` avant commit.
- Python (`tools/`) : `uv`, `uvx ruff check --fix`, `uvx ruff format`.
- Client Bevy : `cd client && source sccache-env.sh && cargo clippy --features dev --all-targets -- -D warnings && cargo test --features dev` (sans `CENT_ANS_MUTE` : le test audio échoue muet) ; `CENT_ANS_MUTE=1 client/tests/smoke.sh` doit passer.
- Détails du client, dumps d'état, scripts de test : `client/README.md`.
- **Ne jamais voler le focus du joueur ni faire de bruit** : tout lancement du client Bevy par un agent passe par `client/run_bg.sh` avec `CENT_ANS_MUTE=1` et `--offscreen` (fenêtre masquée, muet), y compris pour les dumps et scripts. Jamais le binaire lancé en direct avec une fenêtre, jamais d'`osascript … frontmost`. Seul `tools/launch.sh` (le joueur lance le jeu) prend le focus.
- Toute dépense cloud est consignée dans `docs/budget.md` (plafond 50 $ v1).
- Décisions d'architecture : un fichier ADR dans `docs/decisions/NNNN-titre.md`.
- Pas de ligne Co-Authored-By dans les commits.
- Seed bots (ADR 0348) : passe rapide en fin de lot ; passe longue lancée par l'agent en arrière-plan après une grosse fonctionnalité, sans demander (changement du 13/10) ; les constats lui sont présentés, il choisit ce qui entre dans `docs/wip/seed-bots-findings.md`.

## Commandes
- Build du client : `cd client && source sccache-env.sh && cargo build --features dev` (profil release : `tools/launch.sh`)
- Lancer le jeu : `tools/launch.sh` (compile le client Bevy, vérifie le relief composé, ADR 0117/0362) ; essai à blanc : `tools/launch.sh --dry-run`
- Tests : `cd core && cargo test --workspace` ; `cd client && cargo test --features dev` ; `CENT_ANS_MUTE=1 client/tests/smoke.sh` ; `uv run --project tools pytest`
- Assets 3D générés (image → 3D, gratuit d'abord) : `docs/pipeline-assets-3d.md`
- Galerie web des assets générés (images, vues, 3D, prompts) : `nohup python3 tools/experiments/dn_live_gallery.py &` puis http://127.0.0.1:8765/ (`docs/pipeline-assets-3d.md` § 6)
- Seed bots : `cd core && cargo run --profile seed-bots -p seed-bots -- run --quick` (≈ 20-30 s) ; passe longue sans `--quick` (≈ 12 min, en arrière-plan) ; `replay <constat.json>` ; ADR 0348
- Éditeur de la carte composée (crêtes, cols, épingles, lieux, masques) : `uv run --project tools cent-ans compose editor` puis http://127.0.0.1:8766/ ; `docs/editeur-carte.md`
- Animations (mocap vidéo, Muybridge, mesures sur vidéos libres, shaders) : `docs/animation.md`

## Règles de robustesse pour les agents (quota Claude Code)
- Commencer chaque tâche par le squelette (API publique, fichiers vides, tests désactivés) et le commiter avant d'implémenter.
- Écrire un fichier `docs/wip/<tâche>.md` (état, prochaine étape) mis à jour à chaque commit `wip:` pour qu'un agent de reprise sache où continuer.
- Pas plus de 10 agents en parallèle par vague ; les agents d'implémentation bien spécifiés tournent sur un modèle plus léger (Sonnet) quand la tâche est mécanique.

## Vérification visuelle (contexte = quota)
Coûts mesurés (capture 640×400 ≈ 340 tokens ; liste d'UI complète d'un écran chargé ≈ 16 000 tokens). Règles :
- **Texte ciblé d'abord.** Avant toute capture : tests (`cargo test`, `client/tests/smoke.sh`), logs, **dump d'état** (`--dump-state`, `client/README.md`). Une capture ne sert qu'à juger un rendu (UI, 3D, lisibilité), jamais à vérifier ce qu'un test, un log ou un dump établit.
- **Budget : 5 captures par tâche**, 10 au plus si l'utilisateur demande un travail visuel. Au-delà, s'arrêter et dire pourquoi.
- **Jamais en boucle.** Pas de capture après chaque retouche ni pour « attendre » un état (sonder le dump, pas l'image) : regrouper les corrections, puis une seule capture de contrôle.
- **Résolution par défaut (640 px).** Ne pas monter au-delà sans besoin précis de lire un petit texte ; recadrer plutôt qu'agrandir.
- **Dumps filtrés** : ne demander que la section utile du dump d'état ; ne jamais déverser la carte de campagne ou une bataille entière.
- **Clics** : par nom d'élément dans les scripts (`click <nom>`), jamais estimés depuis une capture réduite.
- **Sous-agents :** les agents d'implémentation (Sonnet, tâches mécaniques) ne prennent pas de capture ; la vérification visuelle revient à la session principale ou à un seul agent dédié.
- **Captures de preuve / doc** : `tools/compare_views.sh` (vues de `tools/refshots/views.txt`, rendu hors écran) ou une commande `shot` de script client ; ne lire l'image que si un jugement visuel est nécessaire.
- **MCP hors `.mcp.json` partagé** : ne l'activer que dans une session de travail visuel ; jamais pour changer une règle (les règles vivent dans `core/`).
