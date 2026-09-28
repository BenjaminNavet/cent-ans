# 0113 — Sonde c7a : plancher du trésor de la France abaissé à 15 000 livres (lot RS-M)

Date : 2026-09-28. Statut : accepté. Mesures : `docs/wip/rs-m-c7a.md`.

## Contexte

La sonde release `fifty_turns_on_eight_seeds_stay_in_the_c7a_band` (`crates/ai/tests/m3_grid_ai.rs`,
50 tours IA contre IA, graines 1-8) exige un trésor moyen de la France dans [40 000, 160 000]. Bande
mesurée en C7a, avant TW2 (sièges, sort des places, reconstitution, mercenaires, traditions) et FE
(féodalité, princes d'Empire vassaux, IA féodale). Verte après RS-B (41 889, ae5d94f1), elle est rouge
sur main. Bissection sur les états successifs de main (reflog), budget de la France cumulé ligne à ligne :

| État | France | Sièges/t | Batailles | Cause dominante |
|---|---|---|---|---|
| ae5d94f1 RS-B seul | 41 889 | 1,40 | 52 | — |
| 38954b4f + TW2 SB-T3 | 34 286 | 1,41 | 61 | Philippe VI pris au combat dans plusieurs graines : 12 échéances de rançon de 16 500 livres (≈ 24 700 par partie en moyenne) ; aucune à ae5d94f1 |
| a2672d2e + FE vague 1 | 19 482 | — | — | banqueroute de la France (graine 5) |
| 0cdafa9c + FE vague 2 | 16 601 | 2,70 | 82 | guerre de l'ost d'Empire |
| 5aae540f main (RS-C) | 24 336 | 2,44 | 80 | idem |

Avec FE, dans chaque graine, l'Empire s'allie au Hainaut (lui-même allié de l'Angleterre), déclare la
guerre à la France vers le tour 8 (« défense d'un allié ») et convoque l'ost de ses vassaux directs
déduits : Autriche, Brabant, Gueldre, Hainaut, Confédérés, mais aussi Milan, Savoie, Gênes et Vérone
(`de_jure_liege: tit_empire`). Milan et la Savoie prennent le Lyonnais, l'Auvergne, le Rouergue et le
Nîmois, que la France cède à la paix. Recettes de la France −92 000 sur 50 tours par rapport à 38954b4f.

Contre-épreuve (main, `summon_host` coupé pour le seul Empire, essai non retenu) : sièges 1,95,
batailles 62, recettes revenues à 1 745 719 (> 38954b4f), mais la France convertit ce revenu en armées
(entretien +72 000) : trésor 25 065. Le trésor final mesure la politique de dépense de l'IA plus que
la santé de l'économie ; il varie de 8 000 à 67 000 selon la graine et l'état (sur main : 10 599 à 34 576, écart type ≈ 9 500,
erreur type de la moyenne sur 8 graines ≈ 3 400).

## Décision

1. Pas de bogue du cœur trouvé : la rançon royale est une règle ancienne (fréquence changée par les
   batailles de TW2) ; la guerre de l'ost d'Empire découle de la spec FE § 4.1 (« le vassal doit l'ost
   quand son suzerain direct entre en guerre ») et des titres de FE4b. Sa vraisemblance historique
   (seigneuries italiennes à l'ost impérial contre la France) relève du lot d'équilibre FE F8, déjà
   noté dans `docs/wip/fe.md` ; RS-M ne modifie ni les règles féodales ni les titres.
2. Seul le plancher du trésor de la France dans la sonde change : **40 000 → 15 000** (main 24 336
   moins un peu moins de trois erreurs types, arrondi au multiple de 5 000). Plafond et autres bornes inchangés (Angleterre 20 316 dans
   [5 000, 40 000], sièges 2,44 dans [1,3 ; 8], bloquées 0,11 ≤ 1, débarquements 5,0 ≥ 1). Les
   gardes contre l'effondrement restent : aucune banqueroute, |Δ provinces| ≤ 5, majeurs vivants.

## Conséquences

- La sonde repasse sur main ; elle rougira encore sur une banqueroute ou un trésor sous 15 000.
- F8 doit juger l'ost d'Empire (distance, vassaux italiens de fait indépendants, alliance d'un
  suzerain avec son propre vassal) ; si F8 le restreint, remonter le plancher d'après la mesure.
- Le diagnostic de budget (recettes, armées, bâtiments, administration, Table, autre, ordres) se
  reproduit avec `docs/wip/rs-m-c7a.md` § Méthode.

## Note (2026-09-28, lot FE F8, ADR 0114)

L'ost est restreint aux vassaux à portée (200 km de la capitale du suzerain ou d'une place ennemie) et
le lien féodal ne vaut plus alliance : l'ost impérial contre la France tombe de 34-67 réponses à 13-15
sur 50 tours, sans vassaux italiens. Sonde c7a release (graines 1-8) : France 32 603, Angleterre
13 251, sièges 1,93 / tour, bloquées 0,10, débarquements 4,6, batailles 80,6. **Plancher France
15 000 → 20 000** (moyenne moins environ trois erreurs types, arrondie au multiple de 5 000
inférieur) ; autres bornes inchangées.
