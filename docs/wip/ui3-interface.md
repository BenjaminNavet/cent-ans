# UI3 — interface de campagne, suite (lots U5, U7 fin, U10, U11, U12, U8 de l'audit A3)

Branche : `worktree-agent-ae5685646cc8b59ac` (main + UI2 `worktree-agent-a817c3f698be6306c` fusionnés).
Source : `docs/audit/a3-ui.md` § 3 et § 8 ; suite de `docs/wip/ui2-interface.md`.
Captures : `docs/audit/captures/ui3/` (avant/après par lot). Propriétaire exclusif de
`game/scripts/map/map_ui.gd`. Interface de bataille non touchée (agent UB1).

## Reprise
1. `core/build.sh` puis `godot --headless --path game --import`.
2. Captures : `godot --resolution 1600x900 --path game res://scenes/campaign_map.tscn -- --stage=<étape> --screenshot=<png>`.
   Étapes ajoutées : `general_picker`, `family_tree`, `codex`, `codex_search`, `turn_banner` ;
   fin de tour : `-- --flow-stage=report|alerts --flow-shot=<png>` ; réglages sur un onglet :
   `--flow-stage=settings --flow-tab=Commandes --flow-shot=<png>` ; accessibilité sans toucher au
   fichier du joueur : `--access=colorblind,contrast,motion`.
3. Tests : `godot --headless --path game --script res://tests/ui3_test.gd` (« ui3 OK »), smoke.

## État : terminé
- [x] U5 fin de tour utile : `season_report.gd` (rubriques Vos terres / Trésor / Armées / Constructions
  et recherches / Le monde, pertes ▼ en rouge puis prises ▲ en tête, bouton d'action par ligne :
  « Voir ⌖ », « Ville », « Technologies », « Finances » ; ligne de synthèse du trésor ; revenus bruts
  laissés au journal), `news_interest.gd` (voisins, alliés, ennemis, vassaux, trêves, 5 grandes
  puissances dont seules les grandes nouvelles passent ; réglage `interface/news_filter`, onglet
  Carte), lettres filtrées (motif en infobulle) et bornées au-dessus des pastilles, toast de
  bataille filtré, `end_turn_cluster.gd` (colonne de pastilles libellées avec compteur, 7 types au
  plus, « Autres avis » au-delà), bandeau « Tour des autres factions » (`MapUI.request_end_turn`,
  `FlowController.end_turn_would_proceed`).
- [x] U7 fin : actions InputMap ajoutées (P N R O G L F1 F5 F9), contrôleurs passés aux actions,
  cartouches de touche sur les boutons de la barre, boutons Objectifs (O) et Agents (G)
  (`press_action`), Codex libellé, onglet « Commandes » (disposition auto / AZERTY / QWERTY, fiche
  générée), `shortcut_sheet.gd`, aide F1 générée depuis l'InputMap.
- [x] U10 : lignes de la Cour cliquables (`CourtRow`), une icône par compétence (30 icônes
  game-icons.net ajoutées à `tools/cent_ans_tools/icons_catalog.py`, `cent-ans assets icons`),
  motif des actions grisées (infobulle + ligne sous les boutons, `CharacterSheet.action_blocker`),
  « Sans chef » ouvre le choix du général (`HudController.open_general_picker`,
  `MapUI.show_general_picker`), héritier : cartouche doré, halo et étiquette « HÉRITIER ».
- [x] U11 : `codex_hub.gd`, fenêtre unique « ✠ Codex » (onglets Histoire / Règles, recherche commune
  avec compteurs, onglets parchemin), panneau central de la pile ; `CodexBubbles.window()` rend la
  vue de la fenêtre commune sur la carte ; encyclopédie adoptée (`set_embedded`) ; fiches à
  découvrir marquées ✧, texte d'aide lisible.
- [x] U12 : réglages `access/colorblind`, `access/reduce_motion`, `access/high_contrast` (onglet
  « Accessibilité »), `accessibility.gd` ; symboles ⚔ ⚭ ⚜ ⌛ sur la carte diplomatique (Label3D),
  statut et attitude chiffrée dans la diplomatie, moral ▲ ■ ▼ sur les cartes du bandeau d'ost ;
  animations réduites (caméra en coupe franche, fondus du menu, du chargement, des lettres et du
  bandeau) ; contraste renforcé appliqué au thème parchemin (réversible).
- [x] U8 : `fr_text.gd` (pluriels exacts, dates), plus aucun « (s) » dans l'interface de campagne ;
  dates de sauvegarde et chemins `data/` déjà traités par U1.
- [x] Police de repli pour ₶ et les symboles : Noto Serif (monnaies), Noto Sans Symbols et
  Noto Sans Symbols 2 (sous-ensembles OFL) dans les `fallbacks` des FontVariation du thème.

## Points ouverts
- Couleurs de la carte diplomatique toujours peu visibles et légende coupée par le panneau de
  province (C11, lot U14) ; les symboles daltoniens compensent en partie.
- Le contraste renforcé agit sur le thème parchemin, pas sur les dessins du HUD (`HudStyle`, constantes).
- La fiche de personnage dépasse encore le bas de l'écran à 900 px (actions sous le pli).
- Pas de réaffectation des touches (seulement la disposition AZERTY / QWERTY pour les libellés).
- Estimation de hauteur des lettres (98 px) prudente : parfois une lettre de moins que la place.
