# VIS-R3b : parcellaire typé (état)

Spec : docs/audits/2026-10-29-visuel-campagne.md, lot R3b. Worktree, non fusionné. ADR : docs/decisions/XXXX-vis-r3b-parcellaire-type.md (numéro à attribuer à la fusion).

## Fait
- Données : `data/art/ground_biome_mix.json` (`view.colormap_near_share` 0,12, `view.lum_band_typed` 0,7, bloc `landscapes` : default / openfield / bocage / midi / montagne ; clé `form` par biome), `data/map/agri_landscapes.json` (clé `form`), schémas mis à jour.
- `client/src/campaign/terrain/parcel.rs` : miroir CPU (hash entier, grille `pc_cell`, couronnes, tirage de classe) + chargement des formes + tests.
- `fields.rs` : uniforme `pc0`, `forms_a..e`, `shares` ; forme d'une ligne dans `rows_b.z` ; `landscape_at` pour le dump.
- `terrain.wesl` (regional_ground, fields_apply et helpers `pc_*`) : grille de lanières déformée au lieu du Voronoi (parcelles proches et patchwork lointain), couronnes de soles par le masque de terroir R (aucun échantillonneur de plus), classe de culture tirée par saison, stries de labour anisotropes, haie en pixels (bocage), part de la carte de couleur au plus près en donnée.
- Dump `campaign_terrain.material.fields` : `colormap_near_share`, `typed_forms`, `form_of_row`, `aspect_by_form`, `landscape_at_focus`.
- A/B sur un seul binaire : `CENT_ANS_PERF_OFF=parcel` restaure l'ancien sol.

## Reste
- Mesures critères 1, 3, 4 ; banc A/B (`tools/bevy_bench/ab.sh map vis-r3b "CENT_ANS_PERF_OFF=parcel" ""`).
- clippy / tests complets, pytest des schémas, renommage XXXX de l'ADR à la fusion.
