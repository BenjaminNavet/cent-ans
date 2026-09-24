# Lot R2 — Relief réaliste des champs de bataille — terminé (non fusionné)

Branche : `worktree-agent-a3ec0644f24b4ad29`. ADR : `docs/decisions/0020-relief-des-champs-de-bataille.md`.

## État
- [x] Cœur `core/crates/sim-battle/src/relief.rs` : flux dérivé `RELIEF_STREAM` (tirages d'avant R2
  inchangés), `ReliefStyle::of(terrain)` ; fBm à déformation de domaine + bruit « ridged » (crêtes,
  croupes) ; 1-2 vallons (vers la rivière s'il y en a une, plus profonds vers l'aval, sinon à
  travers un flanc ou le long du champ) ; ruptures de pente (escarpements en collines/montagnes,
  côté défenseur plus haut 3 fois sur 4 ; volées de talus parallèles en plaine/lande/bocage) ;
  micro-ondulations ; plaine alluviale le long de la rivière ; « replat » de chaque ligne de
  déploiement (hauteur moyenne gardée, pente bornée) ; rampe douce qui garantit ≥ 8 m (collines) /
  16 m (montagnes) au défenseur.
- [x] Bois et boue en grappes : ancre (indice conservé, rétrécie à 0,72 r, déplacée vers pente/crête
  pour les bois, vers le creux pour la boue) + lobes + bosquets/flaques dans `forest_parts` /
  `mud_parts` (serde par défaut) ; `in_forest`/`in_mud`/siège les incluent ; `site.rs` :
  `Occupied.parts` évité par villages, haies, fossés (les mares restent tirées des ancres, donc
  dans les creux).
- [x] Pont : `get_terrain` exporte `forests`/`mud` = ancres puis parties.
- [x] Tests `core/crates/sim-battle/tests/relief.rs` (7) : tirages suivants identiques (valeurs
  relevées avant R2), déterminisme, pentes bornées au centre des lignes, amplitude par terrain,
  hauteur du défenseur, bois sur les pentes et boue au creux. Adaptés : `ai.rs` (graines),
  `b6.rs` (empreintes), `f5.rs` (contact cherché pendant la course).
- [x] Rendu : `battle_terrain.gd` (carte de relief 10 m : pente, creux/bosse à 30 et 90 m ; arbres
  sans double comptage dans les recouvrements, lisières plus claires, buissons sur le seul
  pourtour ; horizon : crêtes « ridged » et ondulations qui prolongent le champ) ;
  `battle_ground.gdshader` (normales de détail 3/9/26 m effacées au loin, roche affleurante,
  terre et cailloux des crêtes, herbe grasse/humidité/ombre des creux).
- [x] Captures `docs/img/r2/{avant,apres}_<terrain>{,_proche,_horizon}.png` (7 terrains).
- [x] Smoke : 23 « smoke OK », 0 erreur. cargo fmt/clippy/test verts.

## Mesures (M4 Pro partagé, `--disable-vsync`, 1600×900, `--bench-at=20`, 2 passes)
| Terrain | Avant : moy. FPS (médiane) · prim. · appels | Après |
|---|---|---|
| plaine | 53,2 / 44,8 (60 / 45) · 1,85-1,93 M · 500 | 52,7 / 49,3 (60 / 60) · 1,85 M · 516 |
| montagne | 44,7 / 48,8 (55 / 60) · 2,02 M · 397 | 49,9 / 57,5 (60 / 60) · 2,52 M · 392 |
| forêt | 54,6 / 52,7 (59 / 60) · 1,98 M · 566 | 47,5 / 49,2 (60 / 60) · 2,0 M · 608 |
Machine chargée (≈ 10 agents) : moyennes très bruitées, médianes au plafond de 60 i/s partout.
Coût visible : montagne +0,5 M primitives (rochers sur les pentes, plus nombreuses), forêt +40
appels (bois étalés en lobes sur plus de tuiles d'arbres).

## Points ouverts
- IA : elle ne lit le relief que par la hauteur (`high_ground`) ; sur les plaines ondulées, l'IA
  active ne bat plus un camp passif qu'une fois sur deux (21/32 avant R2, 16-17/32 après, graines
  0-15, armées en miroir) ; `tests/ai.rs` passe désormais sur les graines 1, 4, 6. À reprendre dans
  `ai.rs` (vallons, crêtes, contre-pente, lignes de vue).
- `BattleSetup.relief_roughness` (rugosité lue sur le heightmap de campagne) non fait : la campagne
  (`sim-campaign/battle_request.rs`) n'a pas accès au heightmap réel (données de rendu).
- Boue supplémentaire du site (sol détrempé) et mares restent des disques ronds (`site.rs`).
- Contours de la règle = réunion de disques ; le bruit de lisière n'est que visuel.
- Mesure de FPS à refaire sur machine calme.
