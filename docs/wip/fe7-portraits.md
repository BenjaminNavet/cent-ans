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
requise pour eux. Sur les 62 : 18 sont déjà ≥ 50 ans en 1337, donc **44 variantes âgées
« old »** à générer, ≈ 2,00 $ supplémentaires. Total estimé ≈ 4,82 $, sous 14 $.

Aucune réduction de périmètre nécessaire (souverains, héritiers, consorts et variantes âgées
tous inclus).

## Étapes
1. `cent-ans assets portraits --envelope <n>` par lots (idempotent) → 62 PNG.
2. Ajouter 44 entrées `aged_variants` (band `old`) dans `data/portraits/archetypes.json` pour
   les personnages < 50 ans en 1337, dans l'ordre : souverains (factions jouables) puis
   héritiers/consorts.
3. `cent-ans assets portrait-archetypes --no-archetypes --only <ids...>` par lots.
4. `godot --headless --path game --import`, commit des `.png`/`.jpg` + `.import`.
5. `uv run --project tools pytest -q`.

## Suivi budget
Voir `docs/budget.md` section « Féodalité FE ». Chaque lot = une ligne (coût estimé, coût réel).
