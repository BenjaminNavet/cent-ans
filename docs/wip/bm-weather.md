# bm-weather (lot météo Bevy)

État : nuages + ombres (sprites), pluie/neige (pool d'entités), brume `DistanceFog`, ressource `FogOfWar`, bandeau de saison : portés dans `client/src/campaign/weather/`.
Reste : voir le rapport (masque météo / sol mouillé / brume de vallée = shader de terrain, lot look ; `map_atmosphere.gd` cirrus / rideaux de pluie / aurore ; éclairs d'orage ; fondu du bandeau).

## weather2 (vague 4)
Porté : `sky.rs` (cirrus, rideaux de pluie, brouillard de fleuve, éclairs d'orage ; réglages `data/fx/map_atmosphere.json`), ombres de cumulus par beau temps (`clouds.rs`, `fair_alpha`), fondu des nuées en parchemin (`ZoomView::strategic`), fondu du bandeau (alpha par couleur de nœud).
Reste : aurore boréale (`map_atmosphere.gd`), champ de cumulus du ciel, masque météo / sol mouillé (shader terrain, lot look), vérification visuelle.

## weather3 (vague 5)
Porté : `aurora.rs` (aurore boréale, hiver/soir, `SkyDusk` à alimenter par le lot look), `cumulus.rs` (champ de cumulus du ciel, 3 plans, textures cuites), `focus_clear` (rideaux de pluie et brouillards de fleuve, 8 niveaux de matériaux partagés), texture de pluie fidèle au shader (stries + voile, enveloppe verticale/latérale en couleurs de sommets), rivières lues directement dans `rivers_render.json` (plus de double `WaterData::load`).
Les familles désactivées dans `data/fx/map_atmosphere.json` (aurore, cumulus) s'allument avec `CENT_ANS_ATMOSPHERE_ALL=1` pour les preuves.
Reste : masque météo / sol mouillé (shader terrain), turn_light (dusk), pénalité mer et `wet` des cumulus, focus_clear sur cirrus/cumulus/aurore (plan unique), vérification visuelle.
