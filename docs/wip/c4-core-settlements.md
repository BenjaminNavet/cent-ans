# C4 — refonte du cœur (colonies)

Spec : `docs/design/2026-09-24-echelle-colonies.md` § 2, 4.2-4.6. ADR 0005. Suit C1
(`docs/wip/c1-settlements-skeleton.md`).

## État : terminé (en attente de fusion)

- [x] Données : `settlement_kinds` des bâtiments (schéma + 24 fichiers) ; dans
  `data/settlements/rules.json` (+ schéma) : sections `movement` et `garrison_upkeep_percent` ;
  types `data-model` (`Building::allowed_in`, `MovementRules`, `MovementGraph`).
- [x] `sim-campaign` : `ProvinceState` sans garnison, siège, bâtiments, chantier ni file de
  recrutement ; propriétaire et contrôleur dérivés de la cité (`settlements.rs`) ; économie,
  siège, recrutement et construction par colonie.
- [x] Mouvement sur le graphe des colonies (graphe chargé, ou graphe de repli déterministe).
- [x] Sauvegarde v5 ; toute version antérieure est refusée (« sauvegarde d'une version
  antérieure à la refonte des colonies »).
- [x] IA (`ai_minimal`, `crates/ai`) : cibles = colonies, la cité d'abord.
- [x] Pont Godot, adaptations minimales de `game/` (`location_province`, `path_provinces`,
  position de l'armée prise dans `settlements_px.json`).
- [x] Tests § 7 (`tests/c4_settlements.rs`, refus v4 dans `tests/campaign.rs`) ; fmt, clippy,
  test, `build.sh`, smoke vert (y compris l'ancien échec `projected_income`).

## Décisions

- **Unité de mouvement** : le km de plaine. `points_per_step` = 140 : moyenne du plus court
  chemin terrestre entre les cités de provinces voisines sur le graphe réel (137 sur 299 paires,
  médiane 113). Une saison vaut 3 pas (2 en hiver) × 140. Une arête plus chère que le
  mouvement d'une saison est plafonnée à ce mouvement.
- **Graphe de repli** (quand `settlement_graph.json` est absent) :
  - colonies d'une même province toutes reliées, coût = distance × terrain ;
  - cité-cité entre provinces voisines, plus les 2 paires les plus proches ;
  - ports des `sea_neighbors` reliés, coût = 2 × 140.
- **Revenu** : chaque colonie rapporte, à son contrôleur, l'impôt de sa province calculé avec
  les bâtiments de la cité et les siens, multiplié par sa part de poids. Une colonie assiégée ne
  rapporte rien. Bonus de province complète : +10 %. `TAX_EFFICIENCY` passe de 0,09 à 0,082 pour
  compenser ce bonus (revenu de la France au départ : 27 713).
- **Garnisons hors cité** (`garrison_upkeep_percent`) : ville 15 %, château 25 %, abbaye 10 %,
  village 0 % ; la cité reste à 50 %. Justification historique : milices urbaines, châtelains et
  gardes d'abbaye étaient soldés sur place. Sans cela, les 137 unités de garnison ajoutées
  coûtaient 10 000 de plus par saison et vidaient le trésor de la France en 12 tours.
- **IA** : bonus de cible cité `CITY_TARGET_BONUS` = 25, malus de fortification 3 ; seules les
  cités lèvent des armées avec leur surplus de garnison.

## Points ouverts (C5/C7)

- Si le perdant d'une bataille n'a aucune colonie amie voisine où se replier, il reste sur place
  (m7 : l'Anglais à Saint-Denis).
- L'UI v1 affiche toujours des chemins par province (`find_path_provinces`) ; la sélection
  d'une colonie comme cible sur la carte est à faire (C5/C6).

## Prochaine étape

Fusion par l'orchestrateur.
