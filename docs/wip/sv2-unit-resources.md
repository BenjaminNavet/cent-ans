# SV2 — coûts en ressources des unités

Branche `sv2-unit-resources`. Réutilise le mécanisme B7c (ADR 0053) des chantiers.

## Règle
- Au recrutement, `cost.resources` de l'unité est tiré de l'offre libre de la faction
  (`free_supply`, une unité par province productrice accessible) ; le manque est importé à
  `base_price × resource_import_multiplier` livres, ajouté au coût (pas de refus, comme les bâtiments).
- Les unités tirées sont réservées par la recrue en file (`QueuedRecruit::drawn`) jusqu'à son entrée
  en garnison ; `free_supply` les déduit comme celles des chantiers.
- Pas d'ordre d'annulation de recrutement : rien à rembourser.

## État
- [ ] cœur (orders, state, buildings::free_supply)
- [ ] pont + UI + bulle
- [ ] IA (offre locale suivie au fil des recrues du tour)
- [ ] tests, docs

## Prochaine étape
Implémenter le cœur.
