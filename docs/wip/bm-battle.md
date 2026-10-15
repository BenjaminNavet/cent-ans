# bm/battle : scène et soldats de bataille (Bevy)

État : scène jouable à regarder. Bataille personnalisée par défaut (France/Angleterre, plaines, 12 000 points chacun, composée par `compose_army`), pas fixe de sim-battle (0,1 s, IA dans `BattleSim::step`), terrain en maillage coloré par sommet, caméra TW, soldats en instanciation GPU (4 lots : camp x à pied/monté).

Fichiers : `client/src/battle/scene/{mod,setup,terrain,camera}.rs`, `client/src/battle/units/{mod,instancing}.rs` + `instanced.wesl` (embarqué).

Reste (Godot) : sol texturé splatmap (battle_terrain_splat.gd, battle_ground_textures.gd), arbres/décor/rivières, atmosphère, figurines animées/skinnées (battle_skinned.gd, battle_unit_looks.gd), LOD, bannières (battle_banner_layer, battle_standards), contour de formation, sang/effets, siège, `PendingBattle` à alimenter par campagne/écran de bataille perso.
Piège : `BattleSim` n'est pas `Sync` : `SyncCell`, systèmes en `ResMut<BattleSession>` (un `Mutex` a provoqué un blocage).
