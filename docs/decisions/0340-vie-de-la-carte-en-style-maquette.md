# 0340 — Vie de la carte en style maquette

## Contexte
Le chantier FK (ADR 0122) a peuplé la carte de campagne de figurines (gens, marchands en
charrette, caravanes, troupeaux, scènes). VT2 (ADR 0138) les a passées à l'échelle 1:1, donc
posées seulement sous une distance caméra de 3 unités. La carte généralisée (GC, ADR 0158) a
ensuite fixé un plancher de caméra à 20 unités : les figurines ne s'affichaient plus jamais.
Les moulins, déjà grossis par GC, restaient trop petits pour être lus.

## Décision
En style de lieux `maquette` (défaut), la vie de la carte suit la logique des maquettes : taille
monde grossie et constante, comme les arbres généralisés et les villes.
- Figurines et accessoires FK : échelle 1:1 × `folk_scale` (220 : un homme ≈ 0,55 unité, environ
  un tiers d'un arbre généralisé), posés jusqu'à `folk_range` (90) ; densités × `folk_density` (2) ;
  déplacement ralenti (`folk_speed` 0,5) ; niveau détaillé sous `folk_detail_distance` (35) ;
  aucune instance dans l'emprise d'une maquette.
- Grands chemins : une part `folk_merchant_share` (0,3) des voyageurs devient un convoi marchand
  (charrette, charretier, marchand), les routes commerciales FK étant souvent coupées au tour 1.
- Calque campagne DN (`countryside_layer`, charrettes de vin/laine, muletiers, caravanes de
  chameaux, bouviers, moulins DN) : déjà tenu à taille écran ; taille × `countryside_size` (2)
  pour rester à l'échelle des figurines.
- Moulins (effets) : `windmill_ratio` 0,65 → 1,0, lisibles à côté d'un arbre et d'un village.
- Réglages dans `data/art/town_maquettes.json` § `props`. Le style `real` garde le 1:1 (VT2).

## Conséquences
- Les assets FK générés redeviennent visibles au zoom normal de la campagne.
- Échelle non réaliste assumée (cohérente avec GC) ; densités doublées pour une carte vivante.
- Coût : le réservoir reste plafonné (`pool_cap`), placement inchangé.

## Révision VG2 (2026-10-10) — visibles en vue moyenne
Le joueur veut les figurines lisibles en vue moyenne (rig ≈ 150-450), pas seulement près du
plancher. En maquette :
- `folk_scale` 300 ; au-delà de `folk_screen_from` (25), taille × (distance / 25)^`folk_screen_exponent`
  (0,9) : taille écran presque tenue (≈ 24 px médian à 220 sur 1440 px de large).
- Les semis (grilles des champs, pâtures, forêts ; voyageurs et charrettes par unité de route)
  s'écartent du même facteur arrondi à une puissance de deux (`FolkPool.spread`) : densité à
  l'écran stable, coût de placement borné.
- Portée `folk_range` 450, rayon de placement = distance × `folk_radius_factor` (1,6) ; le palier
  « proche » ne coupe plus le réservoir en maquette.
- Calque campagne DN : portées et plafond de grossissement × `countryside_range` (4).
