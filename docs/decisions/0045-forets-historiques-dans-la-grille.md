# ADR 0045 — Forêts historiques dans la grille de navigation

- Statut : accepté (lot R3)
- Date : 2026-09-25
- Lié à : ADR 0010 (mouvement libre), ADR 0019 (relief et occupation du sol, lot R1)

## Contexte

R1 a remplacé les forêts de `data/map/splat.png` (canal B) par les forêts vers 1340 (KK10 et
55 massifs nommés) et porté la splat à 4096². Pour ne pas changer les règles en cours de route,
`navgrid.py` lisait en priorité une copie figée de l'ancienne splat (`navgrid_splat.png`) : les
armées traversaient la forêt d'Orléans visible sans ralentir, et marchaient à plein pas dans des
bois qui n'existent plus à l'écran. Le joueur veut que les coûts suivent ce qu'on voit.

## Décision

1. **Forêt** : `navgrid.py` lit `splat.png` (R1) et la **moyenne par bloc** à la résolution de la
   grille (2 × 2 pixels de carte par case de 1,44 km) au lieu d'un échantillon d'un pixel sur
   deux. Une case est en forêt (coût 18) quand au moins la moitié de sa surface est boisée.
   `navgrid_splat.png` et le repli sont supprimés.
2. **Lande n'est pas montagne** : depuis R1 le canal A porte aussi les landes (Landes de
   Gascogne, Campine, Lüneburg, Veluwe) ; lu tel quel, il transformait 26 000 cases de lande
   basse en montagne (coût 30). Le critère « roche ≥ 0,45 » ne fait plus une montagne que sur des
   collines (≥ 350 m ou pente ≥ 0,05) ; l'altitude et la pente restent les critères principaux.
3. **Zones humides** (`wetlands.png`, R1) : roselières et eaux libres (R ≥ 0,5) et pays
   d'étangs denses (G ≥ 0,5 : Dombes, Brenne, étangs lorrains) prennent le **coût marais** (25)
   quelle que soit leur altitude, en plus de l'ancienne règle (plat bas d'une province `marsh`).
   Les étangs ne sont **jamais infranchissables** : G est une densité de pays d'étangs (≤ 0,75),
   pas l'emprise d'un étang ; aucune case n'en est couverte. Les prés humides (B) ne ralentissent
   pas (fonds de vallée partout, polders cultivés).
4. **Jamais un mur** : forêt et marais restent franchissables (plus lents) ; aucune case ne
   devient infranchissable (comptage identique avant/après, 2 504 437 cases à 255). Toutes les
   colonies restent reliées à leur masse terrestre (vérification du pipeline).

## Conséquences

- Cases de forêt (18) : 418 681 → 477 578 ; montagne (30) : 149 186 → 129 025 (le bruit de
  l'ancienne splat faisait des taches de « roche » en plaine) ; marais (25) : 17 652 → 19 819.
  Coût moyen d'une case franchissable 14,95 → 15,14.
- Trajets (plaine-km, sonde `core/crates/ai/examples/r3_route_probe.rs`) : Paris → Orléans
  97 → 103 ; Rouen → Paris 97 → 119 ; Calais → Paris 218 → 229 ; Orléans → Bourges (Sologne)
  84 → 105 ; Londres → Douvres 95 → 106 ; Paris → Reims 121 → 140. Aucun tour de plus sur ces
  trajets ; Paris → Lyon passe de 2 à 3 tours d'été, Paris → Dijon gagne un tour d'hiver.
- Sonde IA 50 tours × 8 graines (`m3_grid_ai`) : batailles 58,5 → 63,2 par partie, armées
  bloquées 0,02 → 0,00 par tour, sièges 2,8 → 2,7 par tour, débarquements anglais 4,0 → 4,5.
- L'aperçu du chemin (Godot) lit la même grille par le pont (`find_path_points` →
  `CampaignState::plan_path`) : rien à changer.
- Régénérer `splat.png` ou `wetlands.png` (lot R1) exige désormais de relancer
  `uv run --project tools cent-ans geo navgrid` (le test `test_committed_navgrid_is_up_to_date`
  le rappelle).
