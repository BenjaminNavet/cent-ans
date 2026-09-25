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

Assauts (même sonde) : générique 8/10, 349 s ; Paris 2/10, 706 s ; Rouen 0/10, 318 s.
Paris à surveiller (5/10 avant) : refaire sur 20 graines après le mobilier.

## État

- [x] Mesures avant, sonde.
- [x] Squelette : données, schéma, test pytest, `town.rs`, `props.rs` vide.
- [x] Partie A : ville dense (anneaux d'îlots orientés + rangées le long du rempart ; villes
  emblématiques : rangées le long du rempart et des rues du plan, puis treillis ; église ;
  cheminement sur rectangles ; incendie entre rectangles, réglage 15 m / 0,025).
- [ ] Partie B : mobilier + collisions + figurines.
- [ ] Pont + Godot + captures.
- [ ] Mesures après, ADR, docs.

## Prochaine étape

Partie B : `props.rs` (façades, faubourgs, marché, village), `sim/obstacles.rs` (figurines).
