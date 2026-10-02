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
- [x] 1. Lettrines réelles : mécanisme en place (`FaUi.initial_for`, `Lettrine`), mais **une
      seule initiale retenue** (D, Paris v. 1415, titres non ajustés : « Diplomatie »…).
      Essayées puis écartées sur capture : C Avignon (historiée), C Paris (grille), D Rouen,
      D Angleterre, E Paris 1460 (visage) → lettre illisible à 45-60 px ; P et E anglais à
      l'encre → flous ; fond damassé réel sous la lettre d'or → l'or se perd. Titre de la
      chronique : non traité (aucun gain attendu).
- [x] 2. Ornements : **rinceau réel à la suite des titres à lettrine** (`FaUi.draw_spray`,
      heures anglaises v. 1400) quand le label a de la largeur libre : cour, faction, techniques,
      diplomatie, fiche. Essayés puis écartés sur capture : baguette de lierre (Paris 1415) sous
      le titre du menu (tache confuse sur fond sombre) et sous le titre de la chronique (+22 px
      dans un corps déjà trop court en 720) ; rinceau dans l'en-tête de la chronique (pas la
      place) ; rosettes de coin et acanthe (les fenêtres enluminées ont déjà leur bordure de
      lierre : surcharge) ; initiale réelle C au menu (« ent Ans »).
- [ ] 3. Sceaux de cire
- [ ] 4. Matières (cuir, laiton, bois, velours)
- [ ] Crédits `CREDITS.md`, tests, rapport

## Captures lues
27 (plafond de 30 levé par le coordinateur en cours de lot : « budget illimité de captures »).
Planches ≤ 1280 px, avant/après assemblés.

## Journal
- Squelette : note, données, schéma, outil, test.
- Initiales repérées : presque toutes des D (livres d'heures) ; C (Décret de Gratien, Avignon),
  P (missel de Beauvais), E (Paris v. 1460), petites initiales A/E/M/C/Q (psautier d'Est-Anglie).
