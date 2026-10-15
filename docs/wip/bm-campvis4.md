# bm-campvis4 — contraste du sol de près, brume du matin

## État : en cours
- Fait : paquets TX 2048 (`hi/`, qualité « haute ») chargés comme Godot (`pack_manifest`), albédo encodé en BC1 sur le CPU (`terrain/bc1.rs`), normales Rg8 ≤ 1024 ; couche absente du paquet 2048 (`chestnut_grove`) prise dans le paquet 1k.
- Fait : écart saisonnier TB1 (`season_k`, `k_wild`, `k_forest`, `season_grass/farm/forest`) porté ; dorure d'été limitée aux cultures (avant : tout le sol).
- Fait : ombres de cumulus par beau temps portées dans le shader (RV-C, `cloud_shadow_light`, force 0.4 de `campaign_map.json` `clouds.cumulus_*`, fondu par distance `sync_cumulus`) ; les sprites d'ombre de beau temps sont coupés (`clouds.rs` `FAIR_SPRITES`).
- Fait : occlusion de vallée (RV-D, `data/map/relief_occlusion.png`, binding 26) sur l'albédo et le terme ambiant.
- Mesure Beauce 120 (été, `[330:700,400:880]`) : Godot lstd 17,3 / blk16 13,1 / p5 51 ; Bevy avant 9,8 ; après 15,1 / 11,7 / 51. Saturation encore 86 contre 108.
- Prochaine étape : Beauce 30, puis brume du matin à Paris 25-60.
