# DA1b — Meubles héraldiques dessinés et étendards aux armes du général

Branche : `feat/da1b-meubles-etendards` (worktree `.claude/worktrees/agent-ad8baf6e847fddbc3`),
basée sur main (4c627f4a, DA1 fusionné). ADR : révision de `docs/decisions/0064-armoiries-des-maisons.md`.

## Plan
1. Meubles SVG du domaine public / CC0 / CC BY (Wikimedia Commons, série « Meuble héraldique »)
   vendus dans `data/heraldry/charges/` (+ `SOURCE.md`, manifeste `charges.json`, `CREDITS.md`).
2. Rendu `resvg-py` (dépendance uv) : un masque de couverture par rôle de couleur (corps, armé,
   couronne, trait), recoloré par teinture ; remplace les polygones LION_RAMPANT, LEOPARD,
   EAGLE, CASTLE, guivre, dauphin (v1 factions et v2 maisons).
3. Tests : non-régression par **blasons** (chaque écu contient les teintes attendues) au lieu de
   l'identité à l'octet des écus de faction.
4. Régénérer factions, maisons, bannières (+ bannières de maisons), atlas.
5. EP5 : étendard du général et des unités nobles aux armes de la maison du général.
6. Planche avant/après `docs/img/da1b/`, captures bataille, ADR 0064 § Révision DA1b.

## État
- [x] 1. SVG vendus (`data/heraldry/charges/`, SOURCE.md, CREDITS.md, schéma)
- [x] 2. Rendu par rôles (`heraldic_charges.py`), branché en v1 et v2 ; écartelés de faction (Castille, Hainaut) en v2
- [x] 3. Tests `test_heraldic_charges.py` (non-régression par blasons), bannières de maisons
- [x] 4. Assets régénérés (écus, maisons, bannières + `banners/houses/`)
- [x] 5. EP5 : `_bearer_layers` (battle_standards.gd), `house_arms` dans data/fx/battle_standards.json, `_banner_cloth` ; smoke vert
- [ ] 6. Planche, captures, ADR

## Prochaine étape
Captures bataille (--standard-shot=mounted|foot), planche docs/img/da1b/, ADR 0064 § Révision DA1b.
