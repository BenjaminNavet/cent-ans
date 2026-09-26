# SZ7 — hébergement du relief fin « Cent Ans relief » (ADR 0077)

Branche `worktree-agent-ab262042c73ec57c4` (worktree d'agent), partie de `main`. Lot SZ7 du
chantier `docs/wip/sz-suites-zoom.md`, suite de ZG7b (`docs/wip/zg7b-export-cache.md`).

## Mandat
Outillage seulement : **aucune publication réseau** (ni `gh repo create`, ni `gh release`) —
réservée à l'accord explicite du joueur. Voir ADR 0077.

1. `data/map/relief_hosting.json` + schéma : URL de base versionnée, nom du paquet, version
   courante liée à la cuisson de la pyramide.
2. `cent-ans geo relief-pack [--out DIR]` : archive `.tar` non compressée (zstd testé sur un
   échantillon synthétique, gain nul sur PNG/bin déjà compressés → décision : pas de compression),
   parts < 1,9 Gio, manifeste JSON (version, parts, SHA-256, crédits), streaming, vérif disque.
3. `cent-ans geo relief-fetch [--dest DIR] [--base-url URL] [--from-dir DIR]` : téléchargement
   repris (urllib Range), vérification SHA-256, extraction atomique, puis `relief-all --check`.
4. `ReliefCacheNotice` : propose `relief-fetch` avant `relief-all`.
5. Tests pytest hors réseau (pack + fetch `--from-dir`, reprise, somme fausse, disque insuffisant).
6. Docs (`docs/geo.md`, commandes de publication à la main du joueur) + addendum ADR 0077 si besoin.

## État
- Squelette (schéma, `relief_hosting.json`, modules vides) : fait.
- Fusion de `main` (SZ2, c4064c29) faite : `bake_versions` (lot SZ2, `bake_stamp.py`) est la
  source d'autorité de la version de cuisson des étages 1-3 ; `bake_signature()` la combine avec
  `generated_at` des fleuves/routes fins (pas encore versionnés individuellement).
- `relief_pack.py` : `pack()` (tar non compressé en flux, split < 1,9 Gio, manifeste SHA-256 +
  crédits extraits de `CREDITS.md`), `bump_version_if_rebaked()` (dérive `version` de
  `bake_signature`), `check_free_space()`. Décision compression : zstd -19 testé sur échantillon
  synthétique (PNG 16 bits, blob CAFV) → gain ≈ 0 %, pas de compression (conforme à l'ADR).
- `relief_fetch.py` : `fetch()` (HTTP Range resume via `urllib`, ou `--from-dir`), vérification
  SHA-256 par part, extraction atomique par renommage (parts chaînées, jamais de tar complet
  matérialisé), `default_dest()` (`CENT_ANS_RELIEF_DIR` puis `data/map`).
- CLI : `cent-ans geo relief-pack [--out]`, `geo relief-fetch [--dest] [--base-url] [--from-dir]`.
- `ReliefCacheNotice`/`ReliefCacheStatus` (jeu) : `FETCH_COMMAND` proposé en premier, `REGEN_COMMAND`
  en repli ; texte de l'avis mis à jour ; `zg7b_cache_test.gd` mis à jour et **passe**
  (`godot --headless --path game --script res://tests/zg7b_cache_test.gd`).
- Tests pytest `tools/tests/test_relief_hosting.py` (12 tests : split/checksums, bump de version
  par cuisson de pyramide, crédits, disque insuffisant, fetch `--from-dir` round-trip et
  remplacement, somme fausse/part manquante refusées, HTTP réel avec reprise sur un
  `http.server.ThreadingHTTPServer` local) : **12/12 OK**.
- Docs : `docs/geo.md` (« Hébergement du paquet « Cent Ans relief » », commandes `gh repo create`
  / `gh release create` à la main du joueur) ; addendum ADR 0077 (version dérivée de
  `bake_versions`, décision compression, absence d'archive intermédiaire complète).
- **Non fait** : empaquetage réel de la vraie pyramide (2,4-2,8 Go) — interdit par le mandat
  (disque presque plein) ; publication réseau (`gh repo create` / `gh release create`) — réservée
  au joueur.

## Prochaine étape
Lot terminé côté outillage. `uv run --project tools pytest` (suite complète) : **707 passed,
2 skipped** (7 min 27, machine lente sur les tests géo à base d'images ; rien de cassé par SZ7).
`uvx ruff check`/`format` propres sur les fichiers du lot (le reste des erreurs ruff du dépôt,
`blender_scripts/campaign_trees.py`, est hors lot). Test Godot `zg7b_cache_test.gd` : OK.
Aucun fichier Rust touché (pas de `cargo fmt`/`clippy`/`test` à lancer).

Reste, à la main du joueur : lancer
`uv run --project tools cent-ans geo relief-pack --out <dossier avec assez de place>` sur un
poste avec assez de disque, puis les commandes `gh` de `docs/geo.md` pour publier.
