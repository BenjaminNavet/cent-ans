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
- [ ] 1-6 (squelette seulement)

## Prochaine étape
Télécharger les SVG (limite de débit Commons : script avec attente), écrire le module de rendu.
