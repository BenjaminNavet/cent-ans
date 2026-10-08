# SC — simplification de la base de code (2026-10-08)

Demande du joueur : grand chantier de simplification (moins de lignes, structure, commentaires de
fonctions, doc, design patterns, performance ; passage en Rust des portions GDScript coûteuses).
Mandat (réponses du 08/10) : mécaniques **libres** (simplifier/supprimer si le code y gagne, ADR
à chaque fois) ; supprimer outils one-shot et sondes obsolètes, condenser les wip clos, garder
les ADR ; **20 agents Sonnet par vague** ; toucher aussi les fichiers de la session FL et les
modifs non commitées de main ; **push** en fin de chantier. Coût cloud : 0 $.

Worktree `../gp-sc`, branche `feat/sc` (dylib copiée de main, `data/map/pyramid` en lien).
Modifs non commitées de main au départ sauvegardées dans le scratchpad (`main-dirty.patch`).

## Taille de départ (lignes)
Rust 188 773 · GDScript scripts 124 976 + tests 50 382 · shaders 13 269 · Python 107 987 ·
docs 58 940 (508 notes wip, 163 ADR).

## Méthode
- Vague 0 : audit en lecture seule par zone → lots chiffrés (gain de lignes, risque, vérif).
- Vagues suivantes : un agent `cent-ans-mech` par lot, worktree isolé, ≤ 5 lots Rust
  simultanés (disque : une cible cargo par worktree), commits sur branche, fusion ff-only ici.
- Vérification : `cargo test`, `cargo clippy -D warnings`, pytest ; Godot seulement en headless (scripts de test ciblés par les agents, `smoke.gd` complet aux fusions). **Jamais de lancement fenêtré du jeu ni de capture** (demande du joueur 08/10).

## Vague 3 (patterns) lancée

Vague 2 entièrement fusionnée (battledev, hooks, uikit, audio, testsupport, schemadefs, names, mock, simsplit). Vague 3 : uipatterns (PanelSection, ListMenu, ConfirmDialog), bridge (ctx accessor, convert, GB3/GB4), treaty (Proposal → Treaty{articles}, ADR 0188 si mécanique), rulesdata (Default depuis JSON embarqué, bundled rules), battleperf (Arc fork, grille spatiale, geom), effects (EffectKind, effects.rs, setup_1337, TurnBudget unique, ADR 0189 si mécanique).

## Consigne ajoutée (08/10)

Le propriétaire précise : le chantier n'est pas que de la suppression ; il faut optimiser et appliquer des design patterns. À partir de la vague 3, chaque lot introduit une abstraction (classe de base, trait, table de données, builder, stratégie) et y fait passer tous les sites ; les lots de pure suppression passent après.

## État
- [x] Vague 0 audit (18/20 zones ; MS et DT en cours) → `docs/wip/sc/lots.md` (catalogue des lots par zone)
- [ ] Vague 1 (lancée 10-08) : worktrees `../gp-sc-<lot>`, branches `sc/<lot>` : probes, naval (ADR 0192), simsplit, names, deadfuncs, gtshots, docs, mock, jsondata, tldel, tlschemas
- Vague 1 fusionnée dans feat/sc : docs, probes, deadfuncs, naval (ADR 0192), tlschemas, gtshots, jsondata (DataFile). En cours : simsplit, names, mock, tldel.
- [ ] Vague 2 (lancée 10-08, 6 lots car disque ≈ 35 Go) : battledev, hooks, uikit, audio, testsupport, schemadefs.
- Veto joueur 10-08 : garder l'issue bataille des rencontres (CB2) et les 4 issues de prise + ruines (CB3).
- Réserve : main a des modifs non commitées sur landmarks_v2/towns_1340 (sauvegarde scratchpad main-dirty.patch) → MA1/MA2 (suppression style real / landmarks v2) en attente de vérif.
- Disque : builds avec `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0`, `core/target` supprimé en fin de lot.

## Coordination FL (session parallèle, `../gp-fl`)
Gel jusqu'à la fusion de FL : `game/scripts/map/vegetation.gd`, `campaign_map.gd` (_process /
map.misc), déclutter des villes (settlement), `outbuilding_layer.gd`, life reground,
`terrain.gdshader` + includes, `dev/map_bench.gd`. Tout portage Rust d'un point chaud de la carte
est annoncé à FL avant de commencer. Base FL : d30 p50 31 ms, d150 p50 46 ms (map.misc 8-12 ms).
