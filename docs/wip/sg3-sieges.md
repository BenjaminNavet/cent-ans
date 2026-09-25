# SG3 — Finitions de siège (servants, bombarde, LOD des engins, équilibre de l'assaut)

Branche `worktree-agent-adaa478392d02c5a8`. Suite de SG2 (`docs/wip/sg2-engins.md`, ADR 0023).

## État
- [x] Sonde d'assaut `core/crates/sim-campaign/tests/sg3_assault_probe.rs` (Paris, Avignon, Bruges,
  Calais, Rouen × 10 graines, IA des deux côtés) ; tests non ignorés : porte d'Avignon (graine 11)
  enfoncée en moins de 300 s de pilonnage, murs ≥ 3 × plus longs que la porte à tout niveau.
- [x] Résistance des ouvrages dans `data/rules/siege_works.json` (schéma
  `siege_works_rules.schema.json`, `SiegeWorkRules::bundled()`), plus de constantes dans le code.
- [x] Servants : clips V2 `crank`, `haul`, `load`, `swab`, `push` (`battle_skinned_poses.py`,
  `human.bones.bin` régénéré), figurines `crew_0` (sans arme) et `crew_1` (refouloir `rammer`) ;
  `game/scripts/battle/siege_crew_fx.gd` (MultiMesh par figurine/clip/camp, mode CUSTOM) piloté par
  `SiegeEnginesFx` d'après `reload` / `reload_period` ; poussée du bélier et du beffroi ; placement
  et rôles dans `data/fx/siege_engines.json` (`crew`). Servants rigides de B1 retirés si actif.
- [x] Captures `docs/audit/captures/sg3/` (`game/tests/sg3_siege_shot.gd`, Bruges et Avignon) : bombarde
  au tir (éclair, recul, fumée), écouvillon et charge, treuil et chargeur du trébuchet, poussée du
  bélier et du beffroi, engins au loin. Le servant au refouloir se tient de côté (`rest`) hors du
  souffle quand il ne travaille pas.
- [x] LOD : `<modèle>_lod.glb` (siege_engines.py, pièces nommées gardées, ~ 45-70 % des triangles)
  au-delà de `lod.simple_m` (140 m), pose à 6 Hz et servants cachés au-delà de `lod.far_m` (300 m,
  = `BattleImpostors.DISTANCE`) ; `SiegeEnginesFx.lod_distances()` pour les préréglages (PF1).
  Bélier et beffroi (nœuds de `BattleSiege`) non concernés.

## Équilibre de l'assaut

Réglage (`data/rules/siege_works.json`) :

| Paramètre | Avant (codé en dur) | Après |
|---|---|---|
| PV de la porte (bois) | 250 × (1 + fort.) → 1500 au niveau 5 | 300 + 80 × fort. → 700 au niveau 5 |
| PV d'un pan de mur (pierre) | 500 × (1 + fort.) → 3000 | 1300 + 600 × fort. → 4300 |
| Bélier, équipage complet | 4 PV/s | 5 PV/s |
| Tir d'engin sur un mur | 1,6 × siege_attack | 1,2 × siege_attack |

Au niveau 5, équipage complet : porte en 140 s de pilonnage (avant 375 s) ; un pan de mur sous un
trébuchet seul en 52 tirs, 10 min 24 s (avant 27 tirs, 5 min 24 s) ; ratio mur / porte ≥ 3 à tout niveau.

Sonde (`SEEDS=10 cargo test --release -p sim-campaign --test sg3_assault_probe -- --ignored
--nocapture`, graines 11-20, limite 1800 s ; temps depuis le début de la bataille, premier coup
de bélier vers 120 s). Beaucoup d'assauts sont gagnés par les échelles avant la chute de la porte.

Armées de la démo (sans engins) :

| Ville | Porte tombée avant → après | Porte (médiane, s) avant → après | Victoires assaillant avant → après | Durée médiane (s) avant → après |
|---|---|---|---|---|
| Paris | 1/10 → 1/10 | 530 → 245 | 10/10 → 10/10 | 240 → 240 |
| Avignon | 6/10 → 7/10 | 672 → 298 | 10/10 → 10/10 | 776 → 371 |
| Bruges | 0/10 → 0/10 | — (escalade à ~240 s) | 10/10 → 10/10 | 243 → 243 |
| Calais | 0/10 → 0/10 | — (escalade à ~230 s) | 10/10 → 10/10 | 228 → 228 |
| Rouen | 1/10 → 4/10 | 1225 → 415 | 6/10 → 9/10 | 1312 → 496 |

Avec un trébuchet et une bombarde ajoutés aux assiégeants (`ENGINES=1`) :

| Ville | Porte tombée avant → après | Porte (médiane, s) avant → après | Brèche (médiane, s) avant → après | Victoires assaillant avant → après |
|---|---|---|---|---|
| Paris | 0/10 → 8/10 | — → 391 | 168 → 351 | 10/10 → 10/10 |
| Avignon | 0/10 → 4/10 | — → 352 | 574 → — | 3/10 → 4/10 |
| Bruges | 0/10 → 10/10 | — → 241 | 212 → 311 | 10/10 → 10/10 |
| Calais | 0/10 → 3/10 | — → 284 | — → — | 10/10 → 10/10 |
| Rouen | 0/10 → 4/10 | — → 415 | — → — | 3/10 → 7/10 |

Avant, la brèche venait avant la porte (les murs tombaient plus vite que le bois) ; après, la porte
cède 2 à 5 min après le premier coup, les murs sous deux engins en 4 à 6 min.

## Points ouverts
- Avignon avec engins (6/10 nuls à 1800 s, déjà le cas avant) : l'équipage du bélier est tué
  (huile, carreaux) quand la porte est à 85 %, les engins sont détruits, l'infanterie reste au pied
  du mur sans escalader : défaut de l'IA d'assaut (`plan_siege_attack`), pas de l'équilibre.

## Prochaine étape
Terminé ; à fusionner par l’orchestrateur.

## Points ouverts (servants, LOD)
- Le recul du fût (0,8 m) se lit mal sur une image fixe ; éclair et fumée bien visibles.
- Les servants ne marchent pas (places fixes autour de l’engin), pas de pierre portée à la main ;
  gestes calés sur le rechargement, non sur le tour exact du treuil.
- LOD : réduction modeste (modèles déjà légers) ; bélier et beffroi sans LOD.
