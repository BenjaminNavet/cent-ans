# 0352 — Les champs peints suivent la géographie

Statut : accepté (13/10).

## Contexte
Les champs peints (ADR 0351) tiraient la couleur d'une sole parmi quatre classes fixes (blé, vert,
labour, jachère) au hasard, partout pareil : la Provence et la Beauce avaient le même patchwork, la
vigne et l'olivier n'existaient pas au loin, et le relief n'y jouait aucun rôle (champs sur les
pentes alpines, rien au bord des rivières). Le joueur veut une carte qui tienne compte de la
géographie.

## Décision
- **Couleur par culture.** `crop_far` de `ground_biome_mix.json` donne à chaque culture (matière ou
  repli : rizière, huerta, essart…) sa couleur d'été lointaine et sa classe de saison
  (`season_field` : 0 blé, 1 culture verte, 2 labour, 3 jachère, 4 pré, 5 foin, 6 pérenne). La table
  du sol (`HbGround.build_table`) les range en colonnes 18-29, une par case de culture. Un repli de
  matière garde sa case et sa couleur (la rizière est verte au loin même rendue avec l'orge de près).
- **Une culture par maille.** Parcelle, sole et quartier tirent leur culture avec les poids du
  paysage régional (`hb_maille`, 70 % suivent la culture dominante de la région) ; le proche (matière)
  et le lointain (couleur) prennent la même case, donc la même culture.
- **Relief et eau** (bloc `geo`) :
  - les cultures cèdent aux estives entre 1 100 et 1 700 m et à la pâture sur les pentes de 0,2 à
    0,45 m/m ;
  - vigne sur les coteaux exposés au sud (pente 0,05-0,32), dans la part `coteau_vine` propre à chaque
    biome et paysage (nulle au nord et à l'est, forte en Bourgogne, Bordelais, Méditerranée) ;
  - prés de fauche au bord des rivières (`river_bed`, 2,5 px carte), sur les fonds plats.
- Planche de contrôle : `game/tests/gr_regions_shot.gd` (8 régions × 2 hauteurs).

## Conséquences
- La saison s'applique par culture : vignes et oliviers ne passent plus au labour l'hiver.
- Une règle de relief se règle dans les données ; `geo.enabled: false` revient au rendu de 0351.
- Coût shader : une boucle de 12 lectures de table de plus par sole lointaine (deux au fondu
  soles → quartiers).
