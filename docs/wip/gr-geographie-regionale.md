# GR — la carte de campagne suit la géographie

Mandat (13/10) : « ok pour tout, tu es en autonomie ». Enchaîner les lots sans redemander ;
pas de crédit Meshy sans accord ; pas de push.

## Lots
1. **Couleur lointaine par culture** : chaque culture (blé, seigle, vigne, olivier, pré, rizière,
   essart…) a sa couleur d'été et sa classe de saison (`crop_far` de `ground_biome_mix.json`) ;
   la sole tire sa culture avec les poids du paysage régional, proche et lointain cohérents.
2. **Relief et eau** : prés de fauche en fond de vallée près des rivières, vigne sur les coteaux
   exposés au sud, cultures qui reculent sur les pentes fortes et en altitude (estives).
3. **Régions historiques vers 1340** (agent historien, sources) : `agri_landscapes.json`,
   recuisson de `agri_regions.png`.
4. **Habitat selon le paysage** : groupé en openfield, dispersé en bocage, perché au sud.
5. **Planche régionale** : 8 régions × 2 hauteurs (Beauce, Bretagne, Provence, Bourgogne,
   Flandre, Guyenne, Castille, Angleterre).
6. **Textures de près** (Z-Image local ; Meshy seulement avec accord).

## État
- 13/10 : squelette ; lots 1+2 en cours (shader `hb_ground.gdshaderinc`, table `hb_ground.gd`).

## Prochaine étape
Lots 1+2, puis planche (lot 5), puis 3, puis 4.
