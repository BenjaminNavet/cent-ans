# BR3 — ville de siège dense et mobilier de rue solide

ADR 0047 (`docs/decisions/0047-ville-de-siege-dense-et-mobilier-solide.md`). Branche d'agent
`worktree-agent-ad517ea3d995810c7`. Suite de BR1/BR2 (`docs/wip/br1-batiments.md`).

## Plan

- Données : `data/rules/siege_town.json` (+ schéma `siege_town_rules.schema.json`, test
  `tools/tests/test_siege_town_schema.py`) : îlots, anneaux, rues, chemin de ronde, église,
  faubourgs, emprises du mobilier (valeurs du manifeste), marché, marge des figurines.
- Cœur :
  - `town.rs` : `TownRules`, `Footprint` (rectangle orienté), `Prop`, `PropKind`, `hash01`.
  - `props.rs` : génération déterministe du mobilier (siège : façades, faubourgs, marché ;
    village : devant les maisons).
  - `siege.rs` : `House` gagne une emprise (`length`, `depth`, `yaw`) ; `build_houses` en anneaux
    denses (règles des données) ; `church` ; `props` dans `SiegeWorks`.
  - `siege_layout.rs` : treillis dense, îlots alignés sur les rues du plan.
  - `sim/pathing.rs` : obstacles = rectangles des îlots (+ marge) et mobilier de la place.
  - `sim/obstacles.rs` : repoussement des figurines hors des emprises dans `soldier_poses`.
- Pont : `props`, emprises et église dans `get_siege` ; `props` dans le dictionnaire du village.
- Godot : `battle_siege.gd`, `battle_village.gd` posent maisons et mobilier depuis le cœur.

## Mesures avant (main afd327d4, sonde `tests/br3_assault_probe.rs`, 10 graines, brèche 40 %, fort. 2)

| Ville | Maisons | Victoires assaillant | Durée médiane (s) | Pertes assaillant | Pertes garnison | Maisons brûlées (moy.) |
|---|---|---|---|---|---|---|
| générique | 21 | 8/10 | 339 | 56 | 197 | 4.2 |
| paris | 32 | 5/10 | 801 | 96 | 215 | 17.0 |
| rouen | 36 | 0/10 | 308 | 73 | 155 | 4.0 |

Incendie d'une maison près de la place, 10 min sans combat :

| Ville | Maisons | Touchées après 10 min (moy.) | Part |
|---|---|---|---|
| générique | 21 | 3.8 | 18 % |
| paris | 32 | 2.0 | 6 % |
| rouen | 36 | 9.9 | 28 % |

## Mesures intermédiaires (ville dense, sans mobilier)

Incendie, 10 min : avec l'ancien réglage (portée 30 m, 0,06/période) toute la ville brûle
(100 % / 95 % / 100 %). Balayage (portée × chance) → retenu **portée 15 m, chance 0,025** :

| Ville | Îlots | Touchés après 10 min | Part |
|---|---|---|---|
| générique | 59 | 14.9 | 25 % |
| paris | 61 | 7.3 | 12 % |
| rouen | 78 | 13.2 | 17 % |

## Mesures après (ville dense + mobilier, 20 graines, brèche 40 %, fort. 2, deux IA)

Avant = main afd327d4 recompilé depuis une archive du commit, avec la même sonde.

| Ville | Îlots avant → après | Victoires assaillant | Durée médiane (s) | Pertes assaillant | Pertes garnison | Maisons brûlées (moy.) |
|---|---|---|---|---|---|---|
| générique | 21 → 59 | 17/20 → 17/20 | 332 → 349 | 54 → 46 | 188 → 177 | 4.5 → 6.2 |
| paris | 32 → 61 | 10/20 → 5/20 | 733 → 706 | 93 → 97 | 203 → 187 | 16.5 → 20.9 |
| rouen | 36 → 78 | 0/20 → 1/20 | 305 → 307 | 72 → 96 | 155 → 150 | 4.8 → 6.4 |

Paris : l'assaillant gagne moins (10 → 5/20). Diagnostic (`BR3_VERBOSE=1`) : les défaites sont des
déroutes vers 600-800 s sans jamais tenir la place ; le résultat suit surtout l'incendie (avec
l'ancien feu, 30 m / 0,06, Paris 13/20 mais 37 îlots brûlés sur 59). Compensations essayées sans
effet mesurable : rues du plan et voie de la porte 11 → 14 m (3/20), chemin de ronde 8 → 12 m
(5/20), rue autour de la place 6 → 12 m (5/20) ; autres réglages du feu : 0,035/15 m → 6/20,
0,045/15 m → 8/20 (mais 34-55 % de la ville en feu en 10 min). Gardé : 0,025/15 m (ville générique
inchangée, incendie comparable à avant). Reste ouvert pour l'équilibre (SG3/EQ).

Coût des figurines (`probe_figure_cost`, 57 régiments, 10 423 figurines à 2,5 figurines/soldat,
en plein assaut) : `soldier_poses` 354 µs/image contre 176 µs pour la disposition seule, soit
+178 µs/image (≈ +260 µs extrapolé à 15 000 figurines). Sans obstacle proche, aucun coût (filtre
par cercle englobant).

Captures : avant = `docs/audit/captures/br1/apres/siege-*.png` (BR2) ; après =
`docs/audit/captures/br3/siege-{large,rue,marche,ilots,assaut}.png` (Bordeaux, siège de la
Guyenne, `--no-speech`).

## État

- [x] Mesures avant, sonde.
- [x] Squelette : données, schéma, test pytest, `town.rs`, `props.rs` vide.
- [x] Partie A : ville dense (anneaux d'îlots orientés + rangées le long du rempart ; villes
  emblématiques : rangées le long du rempart et des rues du plan, puis treillis ; église ;
  cheminement sur rectangles ; incendie entre rectangles, réglage 15 m / 0,025).
- [x] Partie B cœur : `props.rs` (façades, faubourgs, marché, village), mobilier de la place dans
  le cheminement (puits exclu : figurines seulement), `sim/obstacles.rs` (repoussement des
  figurines dans `soldier_poses`), tests `tests/br3.rs`. Pont : `props` + emprises dans
  `get_siege`, `props` dans le village.
- [x] Godot : maisons et mobilier posés depuis le cœur (`battle_siege.gd`, `battle_village.gd`,
  `BuildingKit.prop_model/prop_transform`, `add_front_prop` supprimé), captures.
- [x] ADR 0047, codex (`cdx_jeu_incendies`, `cdx_jeu_assaut`), `br1-batiments.md`, fusion de main.

## Vérifications

cargo fmt/clippy -D warnings/test (634 tests, espace de travail, après fusion de main) ; pytest 478 ; smoke Godot : parties bataille/siège vertes ; le smoke complet échoue comme sur main (« Message queue out of memory » pendant la campagne, contourné localement par un override.cfg non commité ; puis « music playlist too short », musiques absentes du dépôt).

## Limites

- Paris : taux de victoire de l'assaillant divisé par deux (voir plus haut).
- Mobilier du village : figurines seulement (les maisons du village ne bloquent pas les régiments,
  règle B5 inchangée).
- Le puits ne bloque que les figurines (le centre de la place est l'objectif des régiments).
- Le repoussement peut aligner des figurines contre une façade dans une ruelle étroite.

## Prochaine étape

Rien d'obligatoire. Fusion par l'orchestrateur ; équilibre de Paris à reprendre avec SG3.
