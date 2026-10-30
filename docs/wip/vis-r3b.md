# VIS-R3b : parcellaire typé (état)

Spec : docs/audits/2026-10-29-visuel-campagne.md, lot R3b. Worktree, non fusionné. ADR : docs/decisions/0814-vis-r3b-parcellaire-type.md.

## Fait
- Données : `data/art/ground_biome_mix.json` (`view.colormap_near_share` 0,12, `view.lum_band_typed` 0,7, bloc `landscapes` : default / openfield / bocage / midi / montagne ; clé `form` par biome), `data/map/agri_landscapes.json` (clé `form`), schémas mis à jour.
- `client/src/campaign/terrain/parcel.rs` : miroir CPU (hash entier, grille `pc_cell`, couronnes, tirage de classe) + chargement des formes + tests.
- `fields.rs` : uniforme `pc0`, `forms_a..e`, `shares` ; forme d'une ligne dans `rows_b.z` ; `landscape_at` pour le dump.
- `terrain.wesl` (regional_ground, fields_apply et helpers `pc_*`) : grille de lanières déformée au lieu du Voronoi (parcelles proches et patchwork lointain), couronnes de soles par le masque de terroir R (aucun échantillonneur de plus), classe de culture tirée par saison, stries de labour anisotropes, haie en pixels (bocage), part de la carte de couleur au plus près en donnée.
- Dump `campaign_terrain.material.fields` : `colormap_near_share`, `typed_forms`, `form_of_row`, `aspect_by_form`, `landscape_at_focus`.
- A/B sur un seul binaire : `CENT_ANS_PERF_OFF=parcel` restaure l'ancien sol.

## Reste
- Mesures critères 1, 3, 4 ; banc A/B (`tools/bevy_bench/ab.sh map vis-r3b "CENT_ANS_PERF_OFF=parcel" ""`).

## Mesures (reprise 10-30, rebasé sur main, captures 1280x800 recadrées sur le centre hors HUD ; vues `vis_r3b_*` ajoutées à tools/refshots/views.txt)
- Critère 1 (k-moyennes 4 classes, dE CIELAB) : Castille 3 paires >= 8 (18, 28, 20) OK ; Normandie OK (27, 33, 41) ; Loire OK. 
- Critère 3 (teinte moyenne, printemps 1337, bande centrale, forêts comprises) : Castille 92 deg (cible 45-70) ÉCART ; Normandie 111 deg (cible 85-105) ÉCART léger. Mesure polluée par les forêts et le blé en herbe de printemps ; à reprendre en été avec un masque sol seul (point ouvert).
- Critère 4 (Loire d 80, automne forcé) : valeur moyenne 0,37 (cible >= 0,38, de peu) ; classes chromatiques : bruns seulement, pas de vert de pré distinct ÉCART (point ouvert, ajuster les parts d'automne de `landscapes`).
- Haies (largeur `hedge_px`) déjà en données ; couleurs `FP_HEDGE` / `HEDGE_COLOR` et multiplicateurs de saison restent codés en dur dans terrain.wesl : reportés à VIS-R7 (touchent aux uniformes partagés hors zone).
