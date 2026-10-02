# FA5 — interface : matériaux et ornements du domaine public (2026-10-02)

Branche `feat/fa-ui`, worktree `../gp-fa-ui` (depuis `feat/fa`). Brutes hors dépôt :
`~/dev/cent-ans-raw/fa/ui/` ; captures de travail : `~/dev/cent-ans-raw/fa/ui-shots/`.

## Consigne de reprise
> Lis ce fichier et `git log --oneline -10`, puis continue au premier point non coché.

## Chaîne
- Découpes décrites dans `data/ui/fa_ui_assets.json` (schéma `data/schemas/ui_fa_assets.schema.json`,
  test `tools/tests/test_ui_fa_assets.py`) ; aucune coordonnée dans le code.
- Outil `tools/cent_ans_tools/fa_ui_assets.py`
  (`uv run --project tools python -m cent_ans_tools.fa_ui_assets [raw_dir] [out_dir]`) → PNG dans
  `game/assets/ui/fa/` + `SOURCE.md`.
- Jeu : `game/scripts/ui/fa_ui.gd` (`FaUi`) lit le même fichier de données.

## Points (ordre du brief, chacun jugé sur capture avant/après)
- [ ] 1. Lettrines réelles (`Lettrine`), titre de la chronique
- [ ] 2. Ornements de bordure (coins / bandeaux en surimpression)
- [ ] 3. Sceaux de cire
- [ ] 4. Matières (cuir, laiton, bois, velours)
- [ ] Crédits `CREDITS.md`, tests, rapport

## Captures lues (budget 30)
4 / 30 — 2 planches-contact des feuillets, 2 planches de repérage des initiales.

## Journal
- Squelette : note, données, schéma, outil, test.
- Initiales repérées : presque toutes des D (livres d'heures) ; C (Décret de Gratien, Avignon),
  P (missel de Beauvais), E (Paris v. 1460), petites initiales A/E/M/C/Q (psautier d'Est-Anglie).
