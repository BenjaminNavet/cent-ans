# UX2 — Premiers pas (barre du haut, tutoriel U15, conseil « que faire maintenant »)

Branche : `worktree-agent-a109e2bd8dafc923f`. Plan d'ensemble : `docs/wip/ux-prise-en-main.md`.
Audit : `docs/audit/a3-ui.md` (C9, C13, U1, U2, lot U15). Captures : `docs/audit/captures/ux2/`.

## Fichiers
- Barre du haut : `game/scripts/map/map_ui.gd` (section « UX2 : libellés de la barre »),
  `game/scripts/map/chronicle_controller.gd` (bouton plus grisé).
- Tutoriel : `game/scripts/ui/tutorial.gd` (placement, surbrillance, sommaire, « Plus tard »),
  `game/scripts/map/tutorial_controller.gd` (`postpone`, `resume`, `jump_to`).
- Conseil : `data/ui/next_hints.json` + `data/schemas/next_hints.schema.json`,
  `game/scripts/ui/next_hint.gd` (choix), `game/scripts/ui/next_hint_card.gd` (encart),
  `game/scripts/map/next_hint_controller.gd` (état, actions), réglage `interface/next_hint`.
- Tests : `game/tests/ux2_test.gd` (« ux2 OK »), `tools/tests/test_next_hints_schema.py`.

## État : terminé (à fusionner)
- [x] Barre du haut : libellés courts (Diplomatie, Chronique, Cour, Techniques, Codex,
  Objectifs, Agents) ; repli en icône seule dans l'ordre `TOP_COLLAPSE_ORDER` quand la barre ne
  tient pas (`MapUI.fit_top_bar`) ; infobulle = nom + touche. 1600 px : tout libellé sauf Agents ;
  1280 px : Diplomatie, Chronique, Cour.
- [x] Chronique : était `disabled` sans décision en attente (d'où le gris) ; désormais style
  normal, infobulle qui dit pourquoi elle est vide et quand elle se remplit, compteur (n).
- [x] C13 : style `focus` du thème déjà distinct (liseré rouge, fond normal) depuis le lot U2 ;
  vérifié sur `docs/audit/captures/u1/c13-confirm-apres.png`, pas de changement.
- [x] Tutoriel U15 : placement automatique (`TutorialOverlay.place_panel`, côté libre, position
  gardée tant qu'elle reste libre), cadre doré pulsé (fixe si `access/reduce_motion`), plus de
  flèche ; sommaire « ☰ Étapes » cliquable ; « Plus tard » (`postpone`, réglage
  `tutorial/postponed`) repris à la même étape par le conseil, l'aide (F1, bouton « Tutoriel pas
  à pas ») ou Menu → Tutoriel.
- [x] Conseil « que faire maintenant » : `data/ui/next_hints.json` (ordre = priorité), encart haut
  gauche, un clic exécute (même chemin que les pastilles de la cloche), croix = masqué jusqu'à la
  saison suivante, réglage `interface/next_hint` (onglet Carte), masqué pendant le tutoriel et
  sous tout panneau.
- [x] Captures `docs/audit/captures/ux2/` (avant / après), stages `next_hint`, `tutorial_toc`.
- [x] Tests : ux2_test, ui3_test, smoke, pytest du schéma.

## Limites
- La touche L reste à l'encyclopédie (déjà liée) : la reprise du guide passe par le conseil,
  F1 ou le menu, pas par L.
- Suite (cartouches) : les boutons libellés portant une touche ont une marge droite = largeur du
  cartouche + 4 px (`MapUI._pad_for_keycap`), comptée par `fit_top_bar`. Plus de lettre mordue ;
  en contrepartie, à 1600 px, « Objectifs » et « Agents » passent en icône seule ; à 1280 px,
  Diplomatie et Chronique gardent leur libellé.
- Le conseil « armée sans ordre » considère toute armée à pleins points de mouvement sans
  chemin ; une armée laissée volontairement en garnison le déclenche (croix pour la saison).
