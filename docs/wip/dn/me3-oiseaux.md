# ME3 oiseaux (branche dn/me3-oiseaux)

Etat : rendu + donnees + test faits (game/tests/me3_birds_test.gd OK).
- `data/map/map_birds.json` (+ schema `map_birds.schema.json`) : 8 especes (corbeau, oie en V, etourneau, mouette, flamant, grue, cigogne, rapace), comportement, habitat (biome, cote, zone humide, ville, altitude), saisons, champ `model` (nom ModelLibrary ; vide = silhouette en V).
- `data/map/wetland_sites_px.json` : centres/rayons des zones humides en px (cuits depuis wetlands.json).
- `game/scripts/map/map_bird_flocks.gd` (remplace les oiseaux de life_ambient.gd), `life_birds.gdshader` (modes orbite/transit/pose/patauge).
Tests : me3_birds_test OK, smoke OK, schemas pytest OK ; cv1_campaign_life_test : 4 echecs "model for level N" (SettlementGrowth.build_model, hors oiseaux, assets absents du worktree). Reste : mesure de frame sur machine calme, branchement des glb bird_* (champ `model`), capture de controle par l orchestrateur.
