# 0351 — Champs peints : soles et patchwork lointain

Statut : accepté (13/10).

## Contexte
Les parcelles Meshy posées en 3D (MX, `dn_fields.json`) étaient jugées non naturelles par le joueur
et débordaient dans les villes 1:1 ; elles coûtaient ≈ 0,7 M primitives. Les champs peints (ADR 0290)
étaient gris-olive de près (matières TX) et disparaissaient au-delà d'une empreinte de 0,8 (fondu vers
la carte de couleur) : invisibles en vue haute.

## Décision
- Parcelles 3D coupées (`dn_fields.json` `enabled: false`) ; les champs sont uniquement peints.
- Soles (assolement) : 80 % des parcelles prennent la culture de leur sole (Voronoi à
  `sole_scale` × la maille des parcelles) ; le patchwork se lit par blocs, comme un finage réel.
- Patchwork lointain : au-delà du fondu des matières, la couleur de culture des soles (puis de
  grands quartiers, `terroir_scale`) reste posée sur la carte jusqu'à `far_end`, pondérée par la
  part cultivée attendue (pas par tirage de parcelle, pour éviter le crénelage), limites assombries.
- De près, la couleur de culture est posée dès le premier zoom (`patch_start` 0) en gardant le grain
  de la matière ; saturation `patch_sat`.
- Réglages dans le bloc `view` de `ground_biome_mix.json`.

## Conséquences
Champs lisibles de la vue proche à la vue régionale (planche `tests/pf_fields_views_shot.gd`).
Le shader du sol fait deux Voronoi de plus au loin (non mesuré : banc sur machine calme).
Les textures de cultures (Meshy ou Z-Image) ne joueraient que sur la vue proche.
