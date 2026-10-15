# bm/ga3 — modèles GA3 en bataille (Bevy)

État : fait. Preuve : `out/bsiege_*.json` (démo siège de Paris) sections `battle_siege.render.{engine_ga3,engine_files,stone_walls,engine_poses}` et `battle_props_ga`.

- Engins : `siege/animation.rs` (`ga3_variant`, `Mover`, `ram_offset`), `siege/engines.rs` : `ga3_trebuchet.glb` / `ga3_ram.glb` (réglages `ga3` de `data/fx/siege_engines.json`, mêmes nœuds nommés que les modèles de base, repli sur le glb de base si le fichier manque ; `CENT_ANS_NO_GA3=1` = `--no-ga3`). Roues (`Wheel_n`, rayon de la variante), roulis de la caisse, poutre du bélier en pendule (`BeamPivot`, `beam_drop` 2,2 de la variante) calée sur les coups.
- Décor : `decor/props.rs` : remplace les `Kit_<modèle>` (maison, église, moulin, tente, puits, charrette ; part du manifeste, pas sous la neige, pas les ruines) par la variante GA3 (ajustement `fit`), 3 LOD à plages de visibilité ; palissades en segments entiers (`site.rs` saute les volumes de palissade quand elles sont posées : 4 lignes).
- Pierre des murs et tours : `siege/stone.rs` + `stone.wesl` : `castle_wall_varriation` pleine résolution en triplanaire monde (0,22 /m, netteté 4, normale 0,8, rugosité 0,92, teinte STONE → STONE_DARK selon les dégâts), comme `BattleSiege._textured`. L'atlas Masonry (couches 512 px, 3 m) reste le repli.

Restes : hash de tirage des variantes différent de `Hash.vec24` (même proportion) ; gravats et rues restent en atlas ; GA3 ram/trebuchet dans props_ga (`wired: false`) = variantes d'engins déjà posées par `models/siege`, pas des props.
