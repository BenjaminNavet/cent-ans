# ADR 0006 — Mouvement libre des armées

Date : 2026-09-24. Statut : accepté. Remplace, dans l'ADR 0005, le choix du déplacement sur le graphe des colonies.

## Contexte

Avec le déplacement de colonie en colonie (ADR 0005, option B), les armées restent sur des rails. Le joueur veut la carte de campagne de Total War : des armées qui s'arrêtent n'importe où, des zones de contrôle et des attaques jouées pendant son propre tour.

## Options

- **Grille de navigation dans le cœur Rust** : grille de coûts de 2048² produite par le pipeline géo, A* dans `sim-campaign`.
- **NavigationServer de Godot** : la règle de mouvement vivrait côté rendu, sans déterminisme ni tests du cœur.
- **Graphe densifié** (routes, hameaux) : toujours des rails.

Pour le tour : exécution immédiate et factions séquentielles (Total War), ou résolution simultanée en fin de tour.

## Décision

Grille de navigation dans le cœur, avec exécution immédiate et tour séquentiel, conformément au choix du joueur. `Army.location` devient une position libre (ou une colonie). Le graphe des colonies reste le squelette de la planification de l'IA. Les traversées maritimes restent de port à port.

Détail : `docs/design/2026-09-24-mouvement-libre.md`.

## Conséquences

- La phase de mouvement de `end_turn` disparaît : les ordres de mouvement et d'attaque sont résolus au moment où ils sont donnés.
- Les IA jouent l'une après l'autre ; les batailles de l'IA contre le joueur sont résolues automatiquement.
- `STATE_VERSION` passe à 6.
- La vision passe d'un masque par province à un masque par case.
- Les anneaux d'atteignabilité par colonie (C5) cèdent la place à une bulle de portée.
