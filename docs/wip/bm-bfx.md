# bm-bfx : effets de bataille (V16-V19, reste « braises »)

Branche `bm/bfx`. État : terminé (preuves : bataille de Crécy, siège de Paris au crépuscule, voir ci-dessous).

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

## Preuves (dumps, offscreen, muet)
- Crécy (`CENT_ANS_BATTLE_SPEED=8`, sans déploiement) : pieux 1024, pavois 756 (3 replantés après pivot), 296 026 traits lancés, 136 chocs de charge ; poussière coupée (sol détrempé, comme Godot).
- Siège de Paris au crépuscule : braises 221 vivantes sur 8 foyers, 10 projectiles de siège (1857 bouffées de traînée, flèche max 36,8 m), flèches enflammées : 44 252 flammes, 3216 petits feux ; obscurité 0,77 (aucune source de camp alimentée : `battle_smoke.sources` = 0, personne n'appelle encore `SmokeSources::add` pour les camps de bataille).
- Une exécution sur trois s'est arrêtée sans message environ 3 s après le début (infrastructure `open -W`, pas un panic) ; relancée, elle passe.
- Avertissement `Mesh index 17 contains no vertices` vu dans toutes les exécutions de bataille ; non attribué (pas confirmé comme venant de ce lot).
