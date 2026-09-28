# FE7 — Portraits (état de travail)

Branche `feat/fe7-portraits` (worktree `/Users/jean_hubert/dev/game_project-fe7`), issue de
`main` a7db2d76. Plafond propre : 15 $ (section « Féodalité FE » de `docs/budget.md`, ADR 0098).

## Périmètre retenu

Dry-run `uv run --project tools cent-ans assets portraits --dry-run` : **62 portraits
manquants** (256×256, `game/assets/portraits/<id>.png`), tous des souverains/héritiers/consorts
des factions FE (F4a-F4e). Coût estimé ≈ 2,82 $ (openai/gpt-5-image-mini, 0,0455 $/image).
Largement sous le plafond de 15 $ : pas besoin de réduire le périmètre de base (souverains +
héritiers + consorts, la totalité des 62 manquants).

Répartition par rôle (calculée depuis `data/characters/*.json`) :
- `ruler` (souverain) : 50
- `heir` (héritier) : 6
- `consort` : 3
- `regent` : 1
- `noble` : 2

Variantes âgées (convention DA2, ADR 0063) : pour chaque personnage dont l'âge en 1337 est
< 50 ans, ajouter une entrée `aged_variants` bande `old` dans
`data/portraits/archetypes.json` (sinon le personnage bascule vers un archétype générique dès
qu'il dépasse 49 ans en jeu — cf. `living_portrait.gd`). Les personnages déjà ≥ 50 ans en 1337
gardent leur portrait fixe indéfiniment (bande max déjà atteinte), donc pas de variante âgée
requise pour eux. Sur les 62 : 13 sont déjà ≥ 50 ans en 1337 (mon estimation manuelle initiale
de 18 était erronée), donc **49 variantes âgées « old »** générées, ≈ 2,23 $ supplémentaires.

Aucune réduction de périmètre nécessaire (souverains, héritiers, consorts et variantes âgées
tous inclus, bien sous le plafond de 15 $).

## Étapes (toutes faites)
1. `cent-ans assets portraits --envelope 3.5` → 62 PNG (2,86 $ réel).
2. 49 entrées `aged_variants` (band `old`) ajoutées dans `data/portraits/archetypes.json`
   (schéma validé).
3. `cent-ans assets portrait-archetypes --no-archetypes --envelope 3.0` → 49 JPG aged (2,26 $
   réel).
4. `godot --headless --path game --import` : 161 fichiers `.import`/`.jpg` générés et commités.
5. `uv run --project tools pytest -q` : 980 passés, 2 skipped (pré-existants).

## État final
Terminé. Cumul FE réel : 5,13 $ (voir `docs/budget.md`), très sous le plafond de 15 $.
Commits : 56f9ef50 (dry-run/scope), c8825f44 (62 portraits de base), 8c679019 (49 variantes
âgées + `.import`).

## Suivi budget
Voir `docs/budget.md` section « Féodalité FE ». Chaque lot = une ligne (coût estimé, coût réel).
