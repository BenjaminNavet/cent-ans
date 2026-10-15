# bm-bunits2 — effets des unités de bataille (Bevy)

Branche bm/bunits2, dossier `client/src/battle/units/`.

## Fait
- Cadavres et blessés (`corpses.rs`, mode `Pick` du shader `skinned.wesl`) : la baisse du nombre de figurines dessinées d'un régiment pose des corps sur les anciennes places (clip de mort selon `loss_cause`, part de blessés qui rampent / s'assoient puis se figent, orientation face au tueur), plafonds de `battle_gore.json`. Réglages lus par `settings.rs`.
- Armes lâchées (`arms.rs`) : épées, boucliers (face peinte à la livrée), armes d'hast, arcs, arbalètes par régiment qui se débande et par blessé ; tampons circulaires ; `oriented.rs` / `oriented.wesl` = tirage instancié à direction complète (flèches, plus tard autres).
- Volées (`volleys.rs`) : chaque `ShotEvent` de `BattleSim::take_shots` devient des traits instanciés (trajectoire parabolique CPU, constantes de `battle_volley.gdshaderinc`), part plantée au sol / pavois / pieux en couche statique. `ShotFeed` garde les tirs de la frame pour l'audio.
- Fumée (`smoke.rs`) : bouffées de couleuvrine, nuage de bombarde, sources durables `SmokeSources::add/remove/set_intensity` (feux de camp, colonnes d'incendie) ; paramètres de `battle_staging.json`.
- Fondu du cycle de mêlée (`cycle_blend_s`) et réactions aux pertes (`hit`, part `melee_hit`) dans le shader des figurines ; bloc de paramètres passé à 121 `vec4`.
- Bannières royales (oriflamme / dragon quand `no_quarter`, saint Georges) et de maison (général + retinue, `heraldry/banners/houses/`), vent de la météo et direction de la graine (`wind.rs`, `BattleWind`) ; les bannières se couchent sous le vent de près.
- Étendards portés (`standards.rs`) : version légère (mât + étoffe du repère au créneau du porte-étendard, couchée au sol quand tombée) ; le repère s'efface de près.
- Dev : `CENT_ANS_UNITS_SPEED=<facteur>[,1]` accélère la bataille (et active l'IA des deux camps).

## Reste (fichiers Godot)
- Sang et membres tranchés (`battle_gore.gd`), impulsion des coups, cheval qui fuit (`battle_soldiers.gd`).
- Flammes des flèches enflammées, pieux et pavois plantés devant les régiments, traits fichés qui s'effacent après 8 s (`battle_volleys.gd`), sons.
- Porte-étendards avec rig propre, musiciens, trophées, étendards tombés rangés (`battle_standards.gd`) ; armoiries de poitrine (FG3).
- Fumée : planche d'animation `fire_smoke.gdshader`, lueur nocturne des feux.
- Maison du général lue dans `data/characters/<id>.json` (pas dans l'état de campagne).
