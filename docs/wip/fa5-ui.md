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

## État après revue de la session principale (2026-10-02)
- **Gardés** : sceaux de cire réels (Met) — sceau rouge en navette devant « Chronique du temps »
  (décisions historiques), sceau rond reteinté rouge sur « Proposer le traité » et devant chaque
  traité signé (ce dernier non vu sur capture) ; boutons d'action de l'accueil en cuir bordé de
  laiton (`FaUi.plate_box`, `FrontEndStyle.style_action_button`).
- **Refusés, désactivés par le catalogue** (`initials` vide, `display.title_spray` vide) :
  l'initiale D réelle (carré brun flou à 45-60 px, rompt l'unité avec les lettrines dessinées)
  et le rinceau à la suite des titres (gris pâle, lu comme une salissure). Les titres sont
  revenus au pixel près à l'habillage d'avant (diplomatie, cour, faction comparées à la
  capture d'origine). PNG correspondants retirés du dépôt.
- **Code conservé** pour de futurs feuillets à initiales simples (lettre nette sur champ uni,
  peu de détail intérieur) : `FaUi.initial_for`, `Lettrine._draw_real_initial`,
  `FaUi.draw_spray`. Pour réactiver, ajouter au catalogue puis relancer l'outil :
  - une source dans `sources` (fichier brut, institution, objet, lieu et date, URL, licence) ;
  - une entrée `initials` : `{"id": "d_paris_1415", "letter": "D", "source": "cma_1953_366_1",
    "box": [716, 1931, 1270, 2435], "size": 128, "mode": "framed"}` (`mode: "matte"` + bloc
    `matte` pour une lettre sans champ peint) ; `display.initial_scale` règle l'agrandissement ;
  - un rinceau : entrée `ornaments` `{"id": "spray_england_1400", "source": "cma_2006_10",
    "box": [691, 86, 1626, 395], "size": 384, "matte": {"low": 0.13, "high": 0.3,
    "background_blur_px": 30, "fill_holes_px": 400, "min_blob_px": 60, "join_px": 1,
    "feather_px": 0.7}}` puis `display.title_spray` = son id ;
  - passer `mipmaps/generate=true` dans le `.import` des nouvelles textures (affichées réduites).
  Sources des deux essais : Cleveland 1953.366.1 (`cleveland/cma_1953.366.1.jpg`, Paris
  v. 1415) et 2006.10 (`cleveland/cma_2006.10.jpg`, Angleterre v. 1400), CC0.

## Essais écartés pendant le lot (sur capture)
- Initiales C Avignon (historiée), C Paris, D Rouen, D Angleterre, E Paris 1460, P et E anglais
  à l'encre : lettre illisible ou floue à 45-60 px ; C réel au menu (« ent Ans ») ; fond damassé
  réel sous la lettre d'or (l'or s'y perd).
- Baguette de lierre sous le titre du menu et sous celui de la chronique ; rinceau dans
  l'en-tête de la chronique ; rosettes de coin et acanthe (surcharge, cadrage raté).
- Cuir rouge gaufré (motif trop présent sous le texte). Non tentés : barre du haut et boutons
  du thème, ferrures, bois, velours, bouton de fin de tour.

## Tests
`fa5_ui_test`, `smoke`, `ui1_lettrine_test`, `vn_ui_720_test` (+ `_b`, `_c`) verts ; pytest
`test_ui_fa_assets.py` (catalogue sans initiale ni rinceau valide et testé, entrées d'initiale
et d'ornement toujours conformes au schéma). Échecs antérieurs à FA5 (mêmes résultats avec les
scripts de `feat/fa`) : `po_ui_test` C3 (5 tailles de police), `q6_ui_test`, `fe_ui_test` ;
`ui1_lettrine_test` saute en silence son contrôle du panneau de diplomatie (erreur de
compilation `SimFacade` en mode `--script`).

## Planches (hors dépôt, `~/dev/cent-ans-raw/fa/ui-shots/`)
`planche_1_diplomatie_avant_apres.jpg`, `planche_2_details_avant_apres.jpg` (avant la revue),
`planche_3_revue_titres.jpg`, `planche_3b_faction.jpg` (après la revue).

## Captures lues
40 (plafond de 30 levé par le coordinateur en cours de lot : « budget illimité de captures »).
Planches ≤ 1280 px, avant/après assemblés.

## Journal
- Squelette : note, données, schéma, outil, test.
- Initiales repérées : presque toutes des D (livres d'heures) ; C (Décret de Gratien, Avignon),
  P (missel de Beauvais), E (Paris v. 1460), petites initiales A/E/M/C/Q (psautier d'Est-Anglie).
