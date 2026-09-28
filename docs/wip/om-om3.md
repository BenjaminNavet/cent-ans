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
- [x] Vérif Godot : import, smoke.gd, ga2_ground, mf1_map_modes, ib_plain, c5_settlements_ui, p2c_ui, om3_terrain_test (nouveau) : OK.

## Points ouverts
- ADR 0116 dit `rel_orthodox` de type `church` ; gardé `other_faith` : dans le modèle, `church`/`obedience` = catholique (is_catholic, excommunication, Schisme). La proximité passe par `kindred`.
- Le climat n'est affiché nulle part côté UI (pas de libellé à ajouter).
- Valeurs d'équilibre (fourrage, charge) à revoir à la sonde de la vague 3.

## Prochaine étape
Lot terminé ; à fusionner dans feat/om. La régénération géo de l'intégration appliquera le coût désert de la grille.
