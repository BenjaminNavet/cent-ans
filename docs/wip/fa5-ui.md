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
- [x] 3. Sceaux de cire réels (Met) : sceau rouge en navette devant « Chronique du temps »
      (décisions historiques, à la place de la croix ✠), sceau rond reteinté en cire rouge sur
      le bouton « Proposer le traité » et devant chaque traité signé de l'onglet Traités
      (ce dernier non vu sur capture : aucun traité signé dans la mise en scène). Bouton de fin
      de tour : non touché (médaillon à cloche déjà en place).
- [x] 4. Matières : plaques 9 tranches cuir brun / cuir cramoisi bordées de laiton
      (`FaUi.plate_box`) pour les boutons d'action des écrans d'accueil
      (`FrontEndStyle.style_action_button` : choix de faction, cartes d'introduction).
      Écarté : cuir rouge gaufré (losanges trop présents sous le texte). Non tenté : barre du
      haut de campagne et boutons du thème (parchemin peint NB, texte à l'encre : un cuir sombre
      imposerait de recolorer tous les textes).
- [x] Crédits `CREDITS.md`
- [x] Tests : `fa5_ui_test` (nouveau), `smoke`, `ui1_lettrine_test`, `vn_ui_720_test` (+ `_b`,
      `_c`), `p2a/p2b/p2d/p2g_ui_test`, `q7_faction_fit_test`, `q8_start_faction_test`,
      `ui3_test`, `dz_diplo_borders_test`, `tw2_t1_capture_test` verts ; pytest
      `test_ui_fa_assets.py` 10 verts. Échecs antérieurs à FA5 (mêmes résultats avec les scripts
      de `feat/fa`) : `po_ui_test` C3 (5 tailles de police, 18 px vue 12 fois), `q6_ui_test`,
      `fe_ui_test` (cadrage du sélecteur de carte) ; `ui1_lettrine_test` saute en silence son
      contrôle du panneau de diplomatie (erreur de compilation `SimFacade` en mode `--script`).
- [x] Planches avant/après (hors dépôt) : `~/dev/cent-ans-raw/fa/ui-shots/`
      `planche_1_diplomatie_avant_apres.jpg`, `planche_2_details_avant_apres.jpg`.

## Reste / pistes
- Le rinceau des titres est discret sur les fenêtres à bordure de lierre (cour, faction) : à
  juger par le joueur ; `display.title_spray` vide dans le catalogue le retire.
- Sceau des traités signés (onglet Traités) : non vu sur capture.
- Autres initiales : il faudrait des feuillets à lettres simples sur champ uni (peu de détail
  intérieur) pour qu'elles se lisent à 45-60 px ; le mécanisme est prêt (ajouter une entrée
  `initials` au catalogue suffit).
- Fusion dans `feat/fa` : à faire par la session principale (`--ff-only` après rebase).

## Captures lues
38 (plafond de 30 levé par le coordinateur en cours de lot : « budget illimité de captures »).
Planches ≤ 1280 px, avant/après assemblés.

## Journal
- Squelette : note, données, schéma, outil, test.
- Initiales repérées : presque toutes des D (livres d'heures) ; C (Décret de Gratien, Avignon),
  P (missel de Beauvais), E (Paris v. 1460), petites initiales A/E/M/C/Q (psautier d'Est-Anglie).
