# OM3 — terrains, climats et religions de l'Est (ADR 0116)

## État
- [x] `Terrain::Steppe`, `Terrain::Desert`, `Climate::Arid`, `Climate::Steppe` (schémas de règles étendus).
- [x] Bataille : profils de décor `steppe`/`desert` (battle_decor.json), largeur de rivière
  (battle_water.json), relief de plaine, bois/villages rares ; auto-résolution (auto_resolve.json :
  steppe charge ×1,1, désert ×0,9).
- [x] Météo de campagne : climats `arid` et `steppe` (campaign_weather.json).
- [x] Mouvement : `terrain_costs.steppe/desert` (movement/rules.json) ; navgrid.py marque les cases
  des provinces de désert (effectif à la régénération géo de l'intégration) ; repli par province.
- [x] Fourrage/attrition : `terrain_supply` (economy.json) : reprise ×70 % / ×35 %, perte ×125 % /
  ×175 %, désert −15 chaque été même en territoire ami.
- [x] Teinte du sol de bataille : `terrain_tints` (data/fx/battle_ground_layers.json).
- [x] Libellés FR (infobulles, panneau de province, dialogue d'avant-bataille, codex).
- [x] Religions : champ `kindred` (orthodoxie ↔ catholicisme = `FaithRelation::Kindred`, opinion −25,
  pas de « guerre de religion ») ; couleurs du filtre religion ; fiche codex.
- [x] Tests : sim-campaign/tests/om3_terrains.rs, sim-battle/tests/om3_terrains.rs.
- [ ] Vérif Godot (import + smoke + tests touchés).

## Prochaine étape
cargo test complet vert, puis dylib (profil om3) → game/bin, import Godot, smoke.gd.
