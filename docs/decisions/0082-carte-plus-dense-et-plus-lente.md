# ADR 0082 — Carte plus dense et plus lente (DC)

Date : 2026-09-26. Statut : accepté (demande du joueur : « la carte est trop petite, les armées
vont trop vite ; je voudrais plus de villes pour une région donnée »). Plan : `docs/wip/dc-densite.md`.

## Contexte

- 570 colonies pour 133 provinces (1 à 6 par province, ADR 0005), soit une colonie tous les
  ~70 km, et un pas de marche vaut justement 70 km (`points_per_step` 140 × `season_scale` 0,5) :
  chaque pas mène d'une place à la suivante, la carte paraît petite.
- La grille logique (4096², 719 m/px) n'est pas le facteur limitant : le rendu descend déjà à
  quelques mètres par pixel (ADR 0036) et deux places à 10 km restent à 14 px l'une de l'autre.
  Diviser les m/px par deux a été chiffré et écarté (≈ 200 constantes, navgrid ×4, sauvegardes
  incompatibles, gain visuel quasi nul).

## Décision

1. **Mouvement ralenti (B)** : l'unité de pas passe de 140 à **70 km** (`movement.points_per_step`).
   Une saison couvre ~105 km de plaine (70 en hiver), la moitié d'avant. Tout ce qui est exprimé en
   pas (repli, portée des agents, horizon de l'IA, traversées) suit ; les distances physiques
   (zone de contrôle 8 km, engagement 5 km, vision 30/20 km, recul 15 km) ne changent pas. Les durées
   en tours (traversée de la Manche, horizon de l'IA) sont vérifiées et préservées.
2. **Colonies densifiées (C)** : de 570 à **~1 200** places attestées en 1337, surtout dans le
   théâtre principal (France, Angleterre, Pays-Bas : 10 à 12 par province), 7 à 8 ailleurs. Plafond
   par province : 16. Les nouvelles places sont surtout des **bourgs (`village`), abbayes et villes**,
   peu de châteaux, pour ne pas refaire l'erreur de C7a (entretien doublé par ~110 places
   secondaires). Poids faibles (bourg 2-5, abbaye 4-8, château 5-10, ville 8-15) ; la cité garde
   son poids.
3. La géométrie de la carte, les provinces et leurs frontières ne changent pas.

## Conséquences

- Données : ~630 nouvelles entrées rédigées et sourcées à la main (même exigence que l'ADR 0005),
  puis régénération `geo settlements`, `geo hamlets`, `geo anchors-fine`, `geo towns`.
- Équilibrage : économie répartie sur plus de places (part de la cité en baisse), garnisons de
  départ, entretien, durée des conquêtes, IA (plus de cibles), fin de tour (`turn_perf`).
- Affichage : 2 fois plus de marqueurs ; niveaux de détail et désencombrement des étiquettes à revoir.
- Codex : les distances citées (210/140 km, un pas = 70 km, repli 280/140 km) sont réécrites.
- Sauvegardes antérieures : colonies inconnues absentes, nouvelles colonies initialisées au chargement
  comme aujourd'hui pour une province sans fichier ; pas de migration dédiée.

## Addendum DC3 (2026-09-26) : équilibrage de la carte densifiée

Mesures `century_probe 120` sur 12 graines, même code que `main` (35783bcf) : voir
`docs/wip/dc-densite.md` § DC3. Trois règles changent :

1. **Horizon de l'IA** : `PLANNING_RANGE` 5 → **10 pas** (`ai/src/campaign.rs`). Le pas de 70 km
   avait divisé par deux le rayon géographique (350 km) : l'Angleterre ne voyait plus que les
   places côtières au-delà de la Manche et les cités prises par décennie tombaient de 40 %.
   L'IA pèse de nouveau 700 km (plus de saisons de marche). L'ancien `OFFENSIVE_RANGE` de l'IA
   minimale (tests) est inchangé.
2. **Effets de province des places secondaires** : `province_effect_percent` (nouveau réglage de
   `data/settlements/rules.json`, schéma `settlement_rules`) : les bâtiments des villes, châteaux,
   abbayes et bourgs pèsent **50 %** sur les effets de toute la province (ordre public, santé,
   biens, ravitaillement, place pour croître), la cité 100 %. Sans cela les ~630 nouvelles
   places (268 églises paroissiales, 102 abbayes de plus) doublaient l'apaisement moyen des
   provinces (−10 → −21), l'IA gardait l'impôt haut 37 % des tours au lieu de 28 % et les
   révoltes disparaissaient. Écarté : « chaque sorte de bâtiment une fois par province », qui
   divise par deux la satisfaction des biens (marchés) et fait lever 16 % d'hommes de moins.
   Les effets propres d'une place (impôt, murailles, recrutement) et le compte des bâtiments
   religieux contre l'hérésie ne changent pas.
3. **Hameaux décoratifs** : à plus de 5 km (au lieu de 3) de toute colonie
   (`geo/hamlets.py`), pour ne plus tomber dans l'emprise d'une maquette.

Gardés : refuge neutre à 1 pas (2 pas essayé : sans effet mesurable, la densité fournit déjà
des refuges) ; garnisons de départ ; bonus de province complète.
