# 0205 — Retrait du village de bataille B5

Statut : accepté (lot SC BB1, 2026-10-09)

## Contexte
Le lot B5 posait sur le champ de bataille un village ou une ferme tiré par la simulation
(`Village`, maisons, clôtures de courtils, ruelles, accessoires BR3). Le décor EP6 (`DecorPlan` :
hameaux, église, manoir, moulins, vergers, camps) couvre le même besoin avec une composition
régionale et une emprise historique. Les deux voies coexistaient : doublon de règles (couvert,
vitesse, charges brisées) et de rendu.

## Décision
- Retrait de `Village`, `Village::speed_factor`, `Battlefield::in_village`, `village_props`,
  `FieldSite::village`, `SiteFeatures::village`, `VILLAGE_COVER`, des clôtures de courtils, du
  couvert « village » de l'IA, de la route du village (hydro) et de la clé `village` de
  `get_terrain()`. `House`/`HouseKind` restent (décor EP6).
- Données : `village_chance` et `timbered_share` (battle_terrain), `village_chance` et
  `village_kinds` (siege_town) supprimés avec leurs schémas.
- `BattleSetup.village: Option<bool>` devient `bare_field: bool` : seul usage restant du
  `Some(false)`, un champ nu (labos, tests, bataille historique) ne pose que les camps.
- Godot : les maisons de `battle_village.gd` disparaissent ; le reste (clôtures, palissades de
  camp, mares, roseaux, mer) vit dans `battle_site_features.gd` (`BattleSiteFeatures`, `SEA_LEVEL`).
  Options `--village` / `--no-village` retirées.

## Conséquences
- Le flux RNG dérivé du site change (plus de tirage village/fermes/courtils) : les batailles
  générées diffèrent des anciennes pour une même graine ; le site ne consomme toujours pas le
  RNG principal. Libellé du site sans « village » / « ferme ».
- Les tests propres au village B5 sont supprimés ; le couvert d'hameau reste testé via EP6.
