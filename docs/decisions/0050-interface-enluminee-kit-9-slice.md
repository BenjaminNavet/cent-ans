# ADR 0050 — Interface « manuscrit enluminé » : kit de textures 9-slice procédural

Date : 2026-09-25. Statut : accepté. Lot UI1.

## Contexte

L'interface n'était faite que d'aplats `StyleBoxFlat` sépia (bande du haut, boutons, panneaux) :
lisible mais triste. Demande du joueur : des assets « moines », enluminures.

## Décision

- **Kit procédural** `tools/cent_ans_tools/ui_illumination.py`
  (`uv run --project tools cent-ans assets ui-illumination`) : Pillow + numpy, déterministe,
  sans licence tierce. Registre des livres d'heures parisiens du XIVe siècle (atelier de Jean
  Pucelle) : vélin, feuille d'or bruni, baguettes azur et gueules à filigranes de blanc de plomb,
  rinceaux à feuilles de lierre, bossettes dorées à quatre-feuilles.
- **Textures 9-slice périodiques** dans `game/assets/ui/illumination/` (+ `kit.json`, marges) :
  le centre de chaque texture est exactement une période du vélin et les motifs des bords ont la
  même période, d'où un pavage sans raccord (`AXIS_STRETCH_MODE_TILE_FIT`).
- **Thème** `parchment_theme.tres` : `StyleBoxTexture` pour panneaux, boutons (enfoncé = feuille
  dorée à filet d'azur, texte à l'encre), onglets, bulles, champs, curseurs, barres de défilement.
  Variations `IlluminatedPanel` (grandes fenêtres : province, cour, fiche, chronique, faction,
  techniques) et `TopBarPanel` (bandeau de campagne).
- **Code** : `HudStyle.kit_box / panel_box / illuminated_box / note_box`,
  `BattleUiKit.page_box / illuminated_box`. `card_box` reste plat (couleurs porteuses de sens).
- **Polices de repli Noto** : métriques verticales ramenées à celles d'EB Garamond (sinon chaque
  ligne du thème faisait 36 px au lieu de 24).

## Conséquences

- Tout écran qui utilise le thème ou `HudStyle` profite du kit sans modification.
- Les marges de contenu comptent pour la mise en page : `panel_box` impose 10 px minimum. Cela a
  révélé une rétroaction dans `diplomacy_panel.gd` (carte ajustée avec une marge fixe de 16 px
  plus petite que le cadre → croissance infinie en fenêtre minuscule) ; corrigée en mesurant le
  cadre réel. Les boîtes de bataille gardent exactement leurs anciennes marges.
- Retoucher le style = modifier le générateur puis relancer la commande ; les marges du thème
  suivent `kit.json`.
