# VIS-R3b : parcellaire typé (état)

Spec : docs/audits/2026-10-29-visuel-campagne.md, lot R3b. Worktree, non fusionné. ADR : docs/decisions/XXXX-vis-r3b-parcellaire-type.md.

## Plan
1. `client/src/campaign/terrain/parcel.rs` : miroir CPU pur (hash entier, cellule grille déformée en lanières, couronnes de soles, tirage de classe par saison) + chargement des formes depuis les données.
2. Données : `data/art/ground_biome_mix.json` (`view.colormap_near_share`, bloc `landscapes.<forme>`), `data/map/agri_landscapes.json` (clé `form`), schémas.
3. `fields.rs` : uniforme (`pc0`, `forms_*`, `shares`), ligne -> forme dans `rows_b.z`.
4. `terrain.wesl` : uniquement `regional_ground` / `fields_apply` et helpers `pc_*` / `season_field_s`.
5. Dump `campaign_terrain.material.fields` : formes + paysage au foyer.
6. Mesures : critères 1, 3, 4 (k-moyennes, HSV), A/B fps.

## État
- squelette en cours.
