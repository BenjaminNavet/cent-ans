# ADR 0253 — Lisibilité de la carte de campagne après la revue d'experts (lot RX mapb)

Date : 2026-10-09. Statut : accepté. Ajuste, sans les remettre en cause, les ADR 0124 (deux vues),
0155 (rouge = ennemis sur bordures, plaques, noms), 0175 (lavis par position), 0236, 0243 et 0244 (sols TX).

## Contexte

La revue RX (`docs/wip/rx/campagne.md`, `campagne-v2.md`) relève sur la vue de jeu (d ≈ 900) : nuages
opaques sur 25-40 % de l'écran, aplat rouge anglais saturé (le lavis RJ-d, pas la teinte de faction du
terrain), noms de petits lieux peu lisibles sur forêt, fleuves du parchemin plus épais que les
frontières, taches « camouflage » au dézoom, rectangle vert à bord droit en Oural (province Grand Perm, bord est) et teinte du sol qui change entre d = 900 et d = 250.

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
- **Parchemin** : les rubans épais étaient les maillages de fleuve (`river_water.gdshader`) et le lit plein de
  `parchment_map.gdshaderinc`. Ruban : encre (0,04/0,10/0,12) → (0,13/0,26/0,33), alpha × 0,75, seul le cœur
  (55 % de la largeur) reste visible ; lit : lavis pâle cerné d'un trait de 0,4 px (avant aplat à 0,7 px).
  Les frontières de royaume (1,3 px) dominent.
- **Oural** : `fog_of_war.desaturation` 0,62 → 0,5, `dim` 0,72 → 0,8 (sol non vu moins terne, voile TB2 gardé). Le rectangle
  vert à bord droit n'est PAS corrigé : un fondu de lisière (`fog_feather_px` 36, flou 5 prises) n'a rien changé à la capture
  et a été retiré. Ce n'est donc pas le brouillard ; la province Grand Perm (n° 289, bord est de la carte, voisine du
  vide id 0) a sans doute une colormap verte dans les données : à examiner avec la re-cuisson de `geo colormap`.

## Conséquences

- Aucune règle de jeu touchée ; données validées par les schémas (`campaign_map_ui.schema.json` étendu).
- Non traité : le brun terne du sol de l'Oural et l'ocre de la colormap (re-cuisson `geo colormap`,
  fichiers `.bin` lourds, hors lot) ; à reprendre dans `colormap_style.yaml` avec une région « est ».
- Captures avant/après : `/private/tmp/claude-501/rx-shots/mapb/` (avant = `campagne-v2`).
