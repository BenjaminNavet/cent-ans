# bm-campvis4 — contraste du sol de près, brume du matin

## État : terminé (lot prêt à fusionner)
- Fait : paquets TX 2048 (`hi/`, qualité « haute ») chargés comme Godot (`pack_manifest`), albédo encodé en BC1 sur le CPU (`terrain/bc1.rs`), normales Rg8 ≤ 1024 ; couche absente du paquet 2048 (`chestnut_grove`) prise dans le paquet 1k.
- Fait : écart saisonnier TB1 (`season_k`, `k_wild`, `k_forest`, `season_grass/farm/forest`) porté ; dorure d'été limitée aux cultures (avant : tout le sol).
- Fait : ombres de cumulus par beau temps portées dans le shader (RV-C, `cloud_shadow_light`, force 0.4 de `campaign_map.json` `clouds.cumulus_*`, fondu par distance `sync_cumulus`) ; les sprites d'ombre de beau temps sont coupés (`clouds.rs` `FAIR_SPRITES`).
- Fait : occlusion de vallée (RV-D, `data/map/relief_occlusion.png`, binding 26) sur l'albédo et le terme ambiant.
- Mesure Beauce 120 (été, `[330:700,400:880]`) : Godot lstd 17,3 / blk16 13,1 / p5 51 ; Bevy avant 9,8 ; après 15,1 / 11,7 / 51. Saturation encore 86 contre 108.
- Beauce 30 (été) : Godot lstd 22,3 / blk16 20,0 ; Bevy avant 8,7 / 5,5 ; après 11,4 / 8,2. L'écart restant tient surtout aux bouquets d'arbres sombres de Godot (absents ou pâles côté Bevy, lot végétation) et à une teinte moins dorée (R/G 1,03 contre 1,15).
- Brume du matin (Paris, météo « brume ») : la brume TB6 n'était pas portée ; le brouillard rendait le sol mouillé (assombri de 8 % et désaturé), d'où un sol gris. Godot ne mouille le sol que sous la pluie et l'orage, avec `wet_dim` 0,04 (Bevy : 0,28). Portés : nappe plafonnée à 7 % (0,2 de près), nappes de vallée TB6 (`morning_mist` de `campaign_map.json`, opacité 0,32, relief lissé `pm_height` et moyenne régionale à 3 × 3 prises), levée au fil du tour (`morning_mist`). Ombres des nuées météo : 0,16 → `weather_shadow` + `mask_shadow` (0,11).
- Paris 60 : moyenne Godot 85,86,73 / sat 62 ; Bevy avant 60,66,48 / 78 ; après 74,79,61 / 65.
- Les « taches pâles » rondes de Paris 25-60 sont présentes aussi par temps clair (écart clair / brume = assombrissement seul) : ce sont les arbres de la végétation, pas la brume.

## Restes
- Prés de fond de vallée et finages 1:1 des villes dans `fp_parcels` ; dessin du brûlis ; moyennes de couches lues dans le manifeste 1k.
- Couverture « humide » des cumulus (`cumulus_cover_wet`) ; province sélectionnée non dégagée de la brume (`wx_selection_clear`) ; normale aplanie sous la nappe de vallée ; météo lue au point visé (pas par province).
- Chargement des paquets 2048 : 8 à 11 s hors fil principal.
- Hors de mes dossiers : arbres pâles (végétation) ; réglages d'image de Godot après tonemapping (contraste 1,06, saturation 1,08) et SSAO à vérifier côté rendu.
