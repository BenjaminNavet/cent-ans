# bm-bfx : effets de bataille (V16-V19, reste « braises »)

Branche `bm/bfx`. État : en cours (voir l'historique git pour le dernier commit `wip:`).

## Fait
- `fx/flipbook.rs` + `.wesl` : sorte `Ember` (braises, additive, `fire_ember.gdshader`), `FlipbookEmitter::embers`, `spawn_fire_sprite` (flammes / braises hors émetteur), `FlipbookLooks::live`.
- `battle/siege/fx.rs` : braises sur les maisons et la porte en feu, fumée de siège poussée par le vent (`BattleWind`), projectiles de siège en vol (pierres, boulets) avec traînée de fumée et impact à l'atterrissage (`Flight`, dump `siege_flights`).
- `battle/effects/` (nouveau) : `dust` (poussière des régiments, colonnes de loin, nuage au contact d'une charge), `glow` (lueur nocturne des feux de camp : lumières ponctuelles selon l'heure).
- `battle/units/volleys.rs` : flèches enflammées (flammes le long du vol et petit feu à l'arrivée), pieux et pavois plantés (`Fieldworks`).
- `battle/units/smoke.rs` : `SmokeSources::glow_points`.
- `main.rs` : enregistrement de `BattleEffectsPlugin` (une ligne).

## Écarts / restes
- Traînée en ruban des flèches (`battle_projectile_trail.gdshader`) : non portée (le lot orienté n'a pas d'alpha) ; seules les pierres de siège ont une traînée (fumée).
- Éclaboussures et sillages aux gués, mottes des sabots, éclair de bombarde : non portés.
- Turbulence des braises (bruit) non portée ; lumières de feux de siège (`siege_fire_fx.gd` `_assign_lights`) : toujours un reste.
- Calibrage de la lueur de camp (lumens) non vérifié à l'œil (pas de capture).
