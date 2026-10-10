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
- Figurines et accessoires FK : échelle 1:1 × `folk_scale` (un homme ≈ 0,3 unité, environ un
  cinquième d'un arbre généralisé), posés jusqu'à `folk_range` ; déplacement ralenti
  (`folk_speed`) pour garder un rythme calme ; aucune instance dans l'emprise d'une maquette.
- Moulins : `windmill_ratio` relevé pour qu'un moulin se lise à côté d'un arbre et d'un village.
- Réglages dans `data/art/town_maquettes.json` § `props`. Le style `real` garde le 1:1 (VT2).

## Conséquences
- Les assets FK générés redeviennent visibles au zoom normal de la campagne.
- Échelle non réaliste assumée (cohérente avec GC) ; densités FK inchangées (par unité de monde).
- Coût : le réservoir reste plafonné (`pool_cap`), placement inchangé.
