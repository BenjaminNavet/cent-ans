# bm-aflags2 : bannières de campagne (restes)

État : bannière au mât de la cogue en mer, éclairage lambert-wrap, décision sur la bannière de maison.

- En mer (`glides.active`), l'étendard n'est plus masqué : pied de hampe à la poupe du navire amiral
  (`STERN_STAFF * SHIP_SCALE - 3`, tourné par le cap du marqueur), comme `ArmyFigures.bearer_anchor` (flotte).
  Écart : pas de tangage/roulis du navire (les cogues ne tanguent pas côté Bevy), donc hampe droite.
- Pastille de posture : déjà portée par `panels/diplomacy/stance_badge.rs` (suit le fleuron) ; elle
  apparaît donc aussi en mer. Rien à ajouter dans armies.
- `flag.wesl` : `diffuse_lambert_wrap` (rugosité 0,9) + BACKLIGHT 0,3, avec la lumière du sol
  (`VegLight` : soleil + ambiance du terrain), repli sans lumière si absente. Pas d'ombre portée.
- Bannière de maison du général : GARDÉE (écart à Godot, amélioration). Même format 256x512 et même
  couverture alpha (0,68) que les bannières de faction : lisibilité identique, et elle distingue les
  seigneurs d'une même faction. Pas d'oeil humain dessus : à valider par le joueur.
