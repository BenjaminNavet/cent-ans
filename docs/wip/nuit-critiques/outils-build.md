# Critique outils / build / données (nuit du 2026-10-22)

Rôle : ingénieur outils. Constats mesurés sur le checkout du 22/10 (main, cfb117c4f). Aucun fichier modifié hors ce rapport.

## 1. Sauvegardes : `version` écrite, jamais vérifiée, pas de migration
- Preuve : `core/crates/game-api/src/saves.rs` définit `SAVE_VERSION: u32 = 1` (commentaire : « `SAVE_VERSION` de `sim_facade.gd` », fichier Godot disparu) ; `grep '\.version'` y donne 0 lecture, `read_save`/`load_state` ne comparent rien, aucun `migrat*`.
- Gravité : haute. Dès que `CampaignState` change (refonte systèmes en cours), une vieille sauvegarde échouera au mieux avec une erreur serde obscure, au pire se chargera de travers.
- Correctif : refuser `version > SAVE_VERSION` avec message FR clair, ajouter une table de migrations `fn migrate(v, Value) -> Value`, et un test « sauvegarde dorée v1 » commitée dans `core/crates/game-api/tests/`. Corriger le commentaire obsolète.
- Effort : M.

## 2. CI : ni pytest, ni fmt, ni clippy, ni validation de schémas ; déclenchement manuel/PR seulement
- Preuve : `grep 'pytest|clippy|fmt' .github/workflows/*.yml` = 0. `client-bevy.yml` ne tourne que sur `workflow_dispatch` et `pull_request` (pas de `push` sur main alors qu'on travaille directement sur main). 226 fichiers de tests Python (dont 101 `*schema*`) ne sont jamais exécutés en CI. CLAUDE.md exige `clippy -D warnings` mais rien ne l'impose.
- Gravité : haute. Le garde-fou « données validées par `data/schemas/` » n'existe qu'en local.
- Correctif : un workflow `checks.yml` sur push main + PR : `cargo fmt --check`, `cargo clippy -D warnings` (core), `uv run --project tools pytest -q -n auto` (ajouter `pytest-xdist`), filtré par `paths` (`data/**`, `tools/**`, `core/**`).
- Effort : M.

## 3. Suite pytest trop lente pour servir de porte
- Preuve : `uv run --project tools pytest -q -x` lancé depuis `tools/` n'était pas terminé au bout de 5 min (passé en arrière-plan, sortie dans le répertoire de tâches de la session, vide à la rédaction). 226 fichiers de tests.
- Gravité : moyenne. Une suite qu'on n'ose pas lancer n'est pas lancée.
- Correctif : marqueurs `@pytest.mark.slow` (tests qui touchent `tools/geo/raw`, Blender, rasters) exclus par défaut ; `-n auto` ; rapport `--durations=20` pour cibler les 10 pires ; viser < 60 s pour le lot rapide.
- Effort : M.

## 4. Schémas JSON : validés seulement côté Python, pas au chargement Rust ni en CI
- Preuve : 258 schémas dans `data/schemas/`, 2403 fichiers JSON/YAML de données ; `jsonschema` n'est dépendance que de `tools/pyproject.toml` ; `grep 'schemas/'` dans `core/` et `client/src` : aucun chargement de schéma (le loader `data-model/src/load.rs` fait du serde). 4 schémas ne sont référencés par aucun test ni code (`compose_paint`, `compose_place_nudges`, `texture_catalog`, `valley_courses`). Les tests de schémas sont écrits un par un (101 fichiers) : un nouveau fichier de données sans test n'est pas détecté.
- Gravité : moyenne à haute (dérive silencieuse données/schéma).
- Correctif : un test unique paramétré qui parcourt `data/**` et vérifie que chaque fichier a un schéma (convention `$schema` ou table `data/schemas/INDEX`) et le valide ; échec si un fichier n'a pas de schéma et n'est pas dans une liste d'exceptions. Brancher les 4 schémas orphelins ou les supprimer.
- Effort : M.

## 5. Résidus Godot non suivis ni ignorés : `game/bin`, `game/shaders`, `game/tests`, 21 `.import`
- Preuve : `git status` : `?? game/bin/` (73 Mo : `cent_ans.release.dll`, `libcent_ans.debug.dylib`…), `?? game/shaders/` (un `.gdshaderinc.uid`), `?? game/tests/` (un `.gd.uid`), 21 `.jpg.import` sous `game/assets/models/council/`. `git check-ignore` ne les couvre pas. Le client Godot a été retiré (ADR 0366). Sur disque : 4451 `.import` au total, 0 suivi. `core/target/debug/build/godot-core-*` encore présent (compilé à l'époque GDExtension).
- Gravité : moyenne (bruit dans `git status`, risque de `git add` accidentel de 73 Mo de binaires, confusion des agents).
- Correctif : supprimer `game/bin`, `game/shaders`, `game/tests`, les `.import`/`.uid` ; ajouter `*.import`, `*.uid`, `game/bin/` au `.gitignore` ; `cargo clean -p godot-core` ou purge de `core/target` (6 Go).
- Effort : S.

## 6. Dépôt : 8,1 Go de packs git pour 695 Mo de fichiers suivis, binaires dans l'historique
- Preuve : `git count-objects -vH` : `size-pack: 8.10 GiO` (5 packs, 158 974 objets) ; `git ls-files` totalise ≈ 695 Mo ; `data/map` seul suivi pèse ≈ 509 Mo (`heightmap.png` 45 Mo, `heightmap_render.png` 45 Mo, `splat.png` 29 Mo…). Pas de Git LFS (`.gitattributes` ne contient que des règles EOL). Le dépôt est public : un clone complet coûte 8 Go.
- Gravité : haute pour tout contributeur ou CI externe (la CI Windows fait un checkout).
- Correctif : (a) `fetch-depth: 1` partout en CI (vérifier) ; (b) mesurer avec `git rev-list --objects --all | git cat-file --batch-check` les 20 plus gros blobs historiques ; (c) décider avec le propriétaire d'un déplacement des rasters vers le paquet relief (déjà hors dépôt, ADR 0149) ou LFS ; une réécriture d'historique exige son go explicite. Ne rien réécrire cette nuit.
- Effort : L.

## 7. Disque : 75 Go dans le checkout, 39 Go de `client/target`
- Preuve : `du -sh` : `client/target` 39 Go, `tools/geo/raw` 14 Go (ignoré), `core/target` 6,1 Go, `export/` 1,9 Go (ignoré), `game/assets/models` 1,9 Go. Plusieurs worktrees/sessions multiplient `client/target` (règle « un build dir par worktree »).
- Gravité : moyenne (disque, vitesse de Time Machine et d'indexation).
- Correctif : script `tools/clean_targets.sh` (`cargo sweep --time 14`, ou `cargo clean` des worktrees supprimés) et rappel dans `docs/tools.md` ; exclure `target/`, `tools/geo/raw`, `export/` de Spotlight et Time Machine (`tmutil addexclusion`) ; profil `dev` : envisager `debug = "line-tables-only"` pour réduire fortement `client/target/debug`.
- Effort : S.

## 8. Dossiers parasites à la racine : `dossier sans titre/`, `out/`, `export/`, `dist/`
- Preuve : `ls` racine montre `dossier sans titre` (créé par Finder), `out` (108 Ko, non ignoré : `?? out/`), `dist` (vide), `pyrightconfig.json` à la racine. `export/` est ignoré mais 1,9 Go et non documenté dans le README.
- Gravité : faible (hygiène).
- Correctif : supprimer `dossier sans titre`, ignorer `out/`, `dist/` ou documenter leur rôle.
- Effort : S.

## 9. Constantes de jeu codées en dur dans `core/` (contraire à « jamais de données codées en dur »)
- Preuve : `grep 'const [A-Z_]+: (f32|f64|u32|i32)'` dans `sim-campaign`, `sim-battle`, `ai` : 209 constantes numériques. Exemples : `sim-battle/src/siege.rs:27-37` (`RING_RADIUS = 150.0`, `GATE_WIDTH = 14.0`, `LADDER_TIME = 45.0`, `TOWER_CLIMB_TIME = 8.0`), `siege.rs:211,215` (`UNDER_ATTACK_*_S`), `relief_ai.rs:25`. L'ADR 0711 a déjà montré la voie (« choc des charges en données »). Elles échappent aux bots de seed, au rééquilibrage par données et aux schémas.
- Gravité : moyenne (le chantier équilibrage / IA de la refonte systèmes en dépend).
- Correctif : un audit scripté (`tools/audit_hardcoded.py` : liste les constantes numériques non triviales par crate) puis migration par lots vers `data/balance/*.json` + schéma, en commençant par `siege.rs` et `ai`. Les constantes purement techniques (EPS, STEP) restent.
- Effort : L (M par crate).

## 10. Lancement : `Lancer Cent Ans.exe` binaire suivi (360 Ko) et 4 lanceurs parallèles
- Preuve : racine : `.bat`, `.command`, `.sh`, `.exe` ; `tools/launch.sh` fait `cargo build --release` chez le joueur (ADR 0117/0153) ; `tools/launcher-windows/` existe. Le `.exe` est un binaire commité : chaque rebuild ajoute 360 Ko à l'historique et aucune CI ne prouve qu'il correspond à `tools/launcher-windows`.
- Gravité : faible à moyenne (reproductibilité, supply-chain pour un dépôt public).
- Correctif : étape CI qui reconstruit le lanceur et compare le hash au `.exe` commité (ou le publie en artefact de release et le retire de git) ; test `tools/launch.sh --dry-run` en CI sur Linux et macOS.
- Effort : M.

## 11. CI Windows/macOS : seul Ubuntu pour le client, aucune mise en cache de `sccache` documentée
- Preuve : `client-bevy.yml` : `runs-on: ubuntu-latest`, `apt-get install` à chaque run, pas de job macOS alors que le développement et le joueur sont sur macOS ; `windows.yml` fait `cargo build --release` du client complet sans cache visible (grep `cache|sccache` à vérifier) ; en local `client/sccache-env.sh` est requis mais le profil `dev` est `opt-level = 1` avec dépendances en `opt-level = 3`.
- Gravité : moyenne (temps de CI, parité macOS non testée).
- Correctif : `Swatinem/rust-cache` ou `mozilla-actions/sccache-action` dans chaque workflow ; un job macOS-14 `cargo test --lib` sur le client ; `cargo nextest` déjà en place à réutiliser.
- Effort : S.

## 12. Index d'ADR : numéros dupliqués
- Preuve : `ls docs/decisions | cut -c1-4 | sort | uniq -d` : 0117, 0146, 0149, 0205, 0357 apparaissent deux fois (324 fichiers, dernier 0711). « Une décision écrite n'est pas rediscutée » suppose qu'on puisse la citer sans ambiguïté ; CLAUDE.md cite « ADR 0357 », « ADR 0117 ».
- Gravité : faible à moyenne.
- Correctif : script `tools/adr_index.py` (existe déjà) étendu en vérification de doublons, appelé par pytest et la CI ; renuméroter les doublons récents en laissant une ligne de redirection dans `INDEX.md`.
- Effort : S.

## 13. `docs/wip` encombré (226 fichiers) et commentaires obsolètes sur Godot
- Preuve : `ls docs/wip | wc -l` = 226 ; le commentaire de `saves.rs` renvoie à `sim_facade.gd` ; `docs/tools.md` (242 lignes) mêle outils actifs et historiques.
- Gravité : faible.
- Correctif : déplacer les wip clos vers `docs/wip/archive/` (mécanisme déjà utilisé côté mémoire) via un script qui lit un champ `statut:`, et un grep CI interdisant `\.gd\b` hors `docs/decisions` et archive.
- Effort : S.

---

## Top 3
1. Saves sans contrôle de version ni migration (constat 1) : risque joueur direct dès le prochain changement d'état de campagne.
2. CI sans pytest/fmt/clippy/validation de schémas, déclenchée seulement par PR (constats 2, 3, 4) : les règles de CLAUDE.md ne sont pas appliquées mécaniquement.
3. Poids du dépôt (8,1 Go de packs, 509 Mo de rasters suivis, constat 6) combiné aux résidus Godot non ignorés (constat 5) : coût de clone et risque de commit accidentel de binaires.
