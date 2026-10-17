# bm-ushape : murs noirs en U dans les champs (campagne d=60)

## Cause (preuve par dump `countryside.frames`, MSM 48,63 N / -1,0 E, d=60)
- Couche : `campaign/countryside` (règles `paddock`, `bocage_enclos` / `bocage_haies_denses`, props
  `wattle_fence`, `hedge_bank`, `stone_wall_limestone`), pas les dépendances de ferme ni les parcelles.
  Dump : 10 800 `wattle_fence` + 5 512 `hedge_bank` instanciés, 10 274 visibles.
- `prop_frame` appliquait aux clôtures le grossissement « maquette » des accessoires
  (`countryside_size` 2, `countryside_range` 4, `art/town_maquettes.json`) : mult = 454 (plafond
  réel 70 x 2 x 4 = 560), soit une clôture de 2 m haute de ~900 m ; segments de 80 m étirés de
  `spread` = 21 (1,7 km). Portée d'affichage 26-42 x 4 = 104-168, donc encore pleinement visibles à d=60.
- Matériau : texture DN `wattle_fence` très sombre (moyenne ~50/255) et `albedo_gain` 1,6 du JSON
  ignoré (Godot : uniform `albedo_gain` de `countryside.gdshader`) : rendu quasi noir.

## Correctif
- `layer::prop_frame` : les props `stretched` (clôtures, haies, murets) gardent le plafond
  `max_mult` du JSON (sans `countryside_size`/`countryside_range`) et leur portée 26-42 non
  élargie : invisibles à d=60, présents aux vues proches. Les autres accessoires (moulins, croix,
  puits) gardent la maquette.
- `layer::tint_props` + `Prop.albedo_gain` : `albedo_gain` appliqué une fois aux matériaux partagés.
- Dump : `countryside.frames` (mult, spread, fade, extent par prop) ; test
  `layer::tests::enclosures_are_capped_and_hidden_from_afar`.

## Écart / à noter
- Godot appliquait la même maquette (même formule) : le défaut en venait du portage fidèle ; écart
  volontaire (mieux que Godot), à confirmer à l'oeil par le joueur.
- Mont-Saint-Michel absent et baie sans eau : données de la carte composée, non traité.

## Preuve après correctif (dumps `countryside`, d=60 sauf mention)
- MSM : visibles 10 274 -> 343 ; wattle_fence mult 454 -> 70, fade 0 ; hedge_bank fade 0.
- MSM d=25 : 7 417 visibles, wattle_fence mult 70 (plafond), fade 1 : les enclos restent aux vues proches.
- Paris et Bordeaux d=60 : `stone_wall_limestone` (964 / 6 926 instances, même défaut latent) fade 0.
