# bm-fxfire : flipbooks de feu et de fumée

État : terminé (voir `client/README.md` § Flipbooks de feu et de fumée).

- `client/src/fx/flipbook.rs` + `flipbook.wesl` : matériau `FlipbookMaterial` (quad billboard, valeurs par sprite dans le `MeshTag`, donc instancié par look), émetteurs de particules, flamme statique de campagne, chronologie des images (pure, testée), dump `fx_flipbook`.
- Crochets : `battle/siege/fx.rs` (`sync_fire_emitters` remplace les boules de flamme/fumée des maisons et de la porte en feu), `campaign/life/effects.rs` (flamme de campagne au lieu des cônes, repli cônes sans plugin), `main.rs` (`FlipbookPlugin`), `lib.rs` (`pub mod fx`).

## Écarts / restes
- Planche de flamme : `fire_flipbooks.json` annonce 16x4, mais le PNG fait 2048x2048 et Godot lit 8x8 : 8x8 retenu.
- Pas de fondu doux au contact (texture de profondeur) ni d'éclairage de la fumée par les lumières de la scène ; pas de braises (`fire_ember`), pas de lumières de feu ; pas de dérive au vent en siège (pas de vent dans `SiegeFrame`).
- Non branchés (dossiers d'autres lots) : fumées de camp et de bombarde (`battle_smoke.gd`, `battle/units/smoke.rs`, `decor/camp_loot.rs`), panaches de campagne (`life_smoke`), fumée des armées en marche ; ils peuvent spawner un `FlipbookEmitter::smoke`. La flamme de feu de camp de Godot (`battle_camp_fire.gdshader`) n'est pas un flipbook : inchangée.
- Flammes de campagne : 0 site de feu au départ de 1337, donc chemin `StaticFlame` compilé mais non vu en dump.

## fxsmoke (suite)
- `fire_flipbooks.json` est correct : `columns`/`rows` décrivent la planche SOURCE (Flame03 16x4), la planche cuite est 8x8 ; schéma et description clarifiés, données inchangées.
- `FlipbookLooks::smoke_tint`, `spawn_smoke_sprite`, `FlipbookEmitter::tint` : fumée teintée.
- Branchés : `battle/units/smoke.rs` (puffs = sprites flipbook), `campaign/life/effects.rs` (bouffées des panaches : quad flipbook), `campaign/armies/camp_smoke.rs` (fumée des camps/bivouacs).
- Restes : braises, lumières de feu de siège, fumée de bataille figée en pause (utilise Time brut), pas de fondu de contact.
