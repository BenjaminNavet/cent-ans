# ADR 0253 — Lisibilité de la carte de campagne après la revue d'experts (lot RX mapb)

Date : 2026-10-09. Statut : accepté. Ajuste, sans les remettre en cause, les ADR 0124 (deux vues),
0155 (rouge = ennemis sur bordures, plaques, noms), 0175 (lavis par position), 0236, 0243 et 0244 (sols TX).

## Contexte

La revue RX (`docs/wip/rx/campagne.md`, `campagne-v2.md`) relève sur la vue de jeu (d ≈ 900) : nuages
opaques sur 25-40 % de l'écran, aplat rouge anglais saturé (le lavis RJ-d, pas la teinte de faction du
terrain), noms de petits lieux peu lisibles sur forêt, fleuves du parchemin plus épais que les
frontières, taches « camouflage » au dézoom, rectangle vert à bord droit en Oural (province amie lavée
en vert, pas une rupture de biome) et teinte du sol qui change entre d = 900 et d = 250.

## Décision (réglages, données d'abord)

- **Nuages** (`data/fx/map_atmosphere.json`, `data/ui/campaign_map.json`) : cumulus `opacity` 0,6 → 0,34,
  `bank_band` 0,66-0,80 → 0,72-0,86 (bancs plus rares), `distance_out` 1000-1250 → 650-950 (fondu selon la
  distance caméra : à d = 900 il reste moins de la moitié) ; nuées météo `max_alpha` 0,45 → 0,30,
  `medium_alpha` 0,20 → 0,14. Étiquettes, plaques et écus sont déjà dessinés sans test de profondeur
  au-dessus ; l'effacement autour du point visé (TB2) est inchangé.
- **Lavis de position** (`data/map/stance_fill.json`) : alphas self/ennemi/ami/autre
  0,50/0,45/0,38/0,10 → 0,34/0,26/0,26/0,08 ; `saturation` 1,0 → 0,5 ; `flat_mix` 0,15 → 0 (plus d'aplat
  qui écrase la luminance). Même réglage pour toutes les catégories : rouge ennemi plus léger, vert ami
  idem ; le rouge plein reste sur frontières, plaques et noms (ADR 0155).
- **Étiquettes** (`labels.outline_px` 4 → 6, `labels.halo_alpha` 0,6 → 0,9 dans `campaign_map.json`) :
  halo de parchemin épais et quasi opaque ; lus par `SettlementLayer`.
- **Parchemin** (`parchment_map.gdshaderinc`) : fleuve à demi-largeur 0,4 px (avant 0,7), alpha 0,7, encre
  bleu-gris plus claire ; les frontières de royaume (1,3 px) dominent.
- **Détail de sol** (`terrain.gdshader`) : le détail de matière s'éteint entre les empreintes 0,45 et 1,3
  (avant 0,7-1,8) ; la teinte propre de couche (`hue_keep`) suit ce fondu (même courbe que le détail), donc
  plus de saut de teinte entre zooms ; l'amplitude de la variation macro de luminance tombe à 35 % entre les
  empreintes 0,6 et 2,0. Le micro-grain TX proprement dit s'éteint déjà à l'empreinte 0,02 : les taches
  « camouflage » venaient du détail de matière et de la luminance macro, pas du micro-grain.

## Conséquences

- Aucune règle de jeu touchée ; données validées par les schémas (`campaign_map_ui.schema.json` étendu).
- Non traité : le brun terne du sol de l'Oural et l'ocre de la colormap (re-cuisson `geo colormap`,
  fichiers `.bin` lourds, hors lot) ; à reprendre dans `colormap_style.yaml` avec une région « est ».
- Captures avant/après : `/private/tmp/claude-501/rx-shots/mapb/` (avant = `campagne-v2`).
