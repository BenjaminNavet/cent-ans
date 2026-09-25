# ADR 0013 — G1 : auto-résolution par phases, calibrée sur la bataille 3D

Date : 2026-09-25. Statut : accepté. Lot N1 de l'audit A2 (`docs/audit/a2-mecaniques.md` § 3 et § 5.2).

## Contexte

L'auto-résolution d'origine (`battle_auto.rs`) additionnait `hommes × attaque`, pondérée par le moral et le terrain ; l'armure ne comptait que contre le tir, la charge et le type d'unité étaient ignorés, et les pertes étaient une fraction uniforme. Résultat : à coût égal, 20 milices battaient 4 chevaliers dans 100 % des cas, alors qu'elles perdent 5 fois sur 6 en bataille 3D. L'IA, qui classe les unités sur ce modèle, ne recrutait que de la milice (99,8 %). Sur 20 affrontements de référence, l'auto-résolution ne donnait le même vainqueur que la 3D que 12 fois (60 %).

## Options

- **Faire jouer la bataille 3D (`sim-battle`) sans rendu** pour chaque bataille de l'IA : fidèle, mais 50 à 500 ms par bataille et des centaines de batailles par tour de campagne.
- **Retoucher la formule scalaire** (armure en mêlée, bonus de charge) : simple, mais sans contres (piques, pieux, écran de front) ni pertes réparties selon l'armure.
- **Résolution par phases et par famille d'unités**, coefficients en données, calibrée sur la 3D par un test.

## Décision

Troisième option. Une bataille auto-résolue se joue en trois phases : salves (3), charge, manches de mêlée (4). Les unités sont rangées en familles (infanterie, piques, tireurs, archers montés, cavalerie, engins) d'après la catégorie, la monture et les capacités de leur type (`UnitProfile::of`). L'armure réduit les pertes à chaque phase ; les piques brisent la charge et frappent les cavaliers plus fort ; les pieux freinent la charge contre des archers en défense ; le front couvre les tireurs contre l'infanterie mais pas contre la cavalerie ; la pluie détend les arcs, la boue ralentit la charge ; le relief renforce le défenseur. Chaque phase coûte du moral en proportion des pertes ; le camp qui rompt perd et subit une poursuite, plus lourde face à des cavaliers. Les pertes du vainqueur et du vaincu sont bornées pour garder la campagne lisible.

Les coefficients vivent dans `data/rules/auto_resolve.json` (schéma `auto_resolve_rules.schema.json`), avec un repli identique dans `AutoResolveRules::default` (vérifié par un test). Le test `core/crates/ai/tests/auto_resolve_calibration.rs` compare le vainqueur de 20 affrontements à des références 3D figées dans une fixture ; la cible est ≥ 80 %, la valeur obtenue 20/20. La sonde `balance_probe rt` régénère les références (`RT_WRITE=1`).

Pour ne pas gêner la refonte du mouvement libre (M2), le seul changement dans `movement.rs` est l'appel : `resolve_field(state, data, attaquants, défenseurs, côtés, contexte, province)` remplace `resolve_auto(...)`. `resolve_field` reconstruit les profils depuis les armées et tire la météo de la saison. `resolve_auto` garde sa signature (profils devinés, règles par défaut) ; les assauts et les sorties de garnison (`siege.rs`) l'utilisent toujours, ce qui suffit derrière des murailles où la charge ne compte pas.

## Conséquences

- La milice ne bat plus la cavalerie lourde ; les chevaliers écrasent l'infanterie légère et échouent sur les piques (Courtrai) ; les archers longs dominent l'infanterie à découvert.
- L'IA ne change pas de recrutement tant qu'elle classe les unités par « puissance par livre » : c'est l'objet du lot E1 (doctrines).
- Les archers longs sont très rentables à budget égal (95 % de victoires moyennes dans la matrice) : à rééquilibrer par les coûts (lot E8).
- La campagne tire un nombre aléatoire de plus par bataille (météo) : les parties d'une même graine diffèrent de celles d'avant N1.
- Reste à faire : passer les assauts par `resolve_field` après la fusion de M2 (qui modifie `siege.rs`).
