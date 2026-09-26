# DA1b — Meubles héraldiques dessinés et étendards aux armes du général

Branche : `feat/da1b-meubles-etendards` (worktree `.claude/worktrees/agent-ad8baf6e847fddbc3`),
basée sur main (4c627f4a, DA1 fusionné). ADR : section « Révision DA1b » de
`docs/decisions/0064-armoiries-des-maisons.md`.

## État : terminé, en attente de fusion par l'orchestrateur DA
- [x] 12 SVG de Commons (domaine public, CC0, CC BY 4.0) dans `data/heraldry/charges/`
  (`SOURCE.md`, `charges.json` + schéma `heraldry_charges.schema.json`), crédités dans `CREDITS.md`.
- [x] `tools/cent_ans_tools/heraldic_charges.py` : rendu `resvg-py` par rôle de couleur, recadrage,
  cache ; `heraldry.py` : `paint_charge`, `charge_paints`, `lion_variant`, `armed_tincture`,
  `crown_tincture` ; branché en v1 (factions) et v2 (maisons) ; écartelés de faction en v2.
- [x] Tests : `tools/tests/test_heraldic_charges.py` (schéma, licences, rôles, non-régression par
  blasons), bannières de maisons ; pytest 636 passés.
- [x] Assets régénérés : écus (29 + 51), bannières (87 + 153 dans `banners/houses/`).
- [x] EP5 : `BattleStandards._bearer_layers` + `is_house_retinue` ; `house_arms` dans
  `data/fx/battle_standards.json` (+ schéma) ; `battle_scene.gd` passe les maisons et
  `_banner_cloth` suit ; `--standard-side=<attacker|defender>` pour les captures.
- [x] Planches `docs/img/da1b/planche_ecus_avant_apres.jpg`, `planche_bannieres_avant_apres.jpg` ;
  captures `etendard_general_monte.jpg` (Valois), `etendard_general_pied_angleterre.jpg`
  (Plantagenêt, `--standard-shot=foot --standard-side=defender`), `etendards_retenue_pied.jpg`.
- [x] Smoke vert.

## Notes
- `--standard-shot=foot` côté France cadre derrière une pile du pont (cadrage d'EP5, pas DA1b) :
  capture pied prise côté anglais.
- Téléchargements Commons : limiter le débit (429), User-Agent descriptif.

## Suites possibles
- Dauphin vif (dessin pâmé faute de mieux), lion de saint Marc « en moleca ».
- Surcot du porte-étendard du général aux armes de sa maison (matériau partagé par camp).
