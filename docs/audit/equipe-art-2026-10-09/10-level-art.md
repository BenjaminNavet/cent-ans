# Level artist — état des lieux (09/10, lecture seule)

## 1. État actuel
**Génération du champ de bataille (Rust)**
- Profils par terrain : `data/rules/battle_terrain.json` → `sim-battle/src/terrain_rules.rs` (9 terrains).
- Relief `relief.rs` (ADR 0020) : fBm, crêtes, vallons, talus ; `defender_rise` seulement en collines (8) et montagnes (16).
- Eau, gués, ponts, routes : `hydro.rs` + `battle_water.json` ; site saisonnier `site.rs`.
- Décor (ADR 0061) `decor.rs`, `decor_gen/` : hameaux, moulins, église, manoir, vergers, vignes, camps ; 13 profils régionaux × 4 saisons ; effets de jeu (couvert, vitesse, charges brisées).
- 3 cartes historiques (Crécy, Poitiers, Azincourt) sur relief réel.

**Rendu** : `battle_terrain.gd` + `_splat/_mesh/_scatter` ; hauteurs/rivière en Rust (lot SC BT7). Végétation `battle_vegetation.gd`, `battle_trees.gd` ; horizon `battle_horizon.gd` (ADR 0032) ; clôtures, mares, décor, ponts.

**Sièges** : `siege.rs`, `siege_layout.rs` ; cas générique = cité octogonale (`siege_town.json`) ; 7 villes à vrai plan (`data/landmarks/`) ; barres de santé, visée (ADR 0107), points de capture.

**Lisibilité** : minimap (relief, bois, boue, obstacles, eau, routes) ; vue tactique = calque assombri.

## 2. Forces
- Décor à effets de jeu, déterministe, en données : ce qu'on voit compte.
- Variété régionale et saisonnière rare ; cartes historiques sur DEM ; horizon par province.
- SC simplifie la chaîne (1 voie de décor).

## 3. Faiblesses
1. Sièges peu variés : ADR 0126 retirée (BB14), toute place sans plan devient la même cité octogonale.
2. Orléans sans plan de siège (`landmarks_v2` sans bloc `siege`).
3. Lisibilité tactique faible : sol olive uniforme (`docs/img/po/po4/01-matin.jpg`) ; minimap sans hameaux/vergers/vignes/camps ; rien sous le curseur (`decor_effect` non lu côté Godot) ; vue tactique sans couche de terrain.
4. Reliefs répétitifs : une rivière, 1-2 vallons ; pas de position défensive en plaine (cœur du jeu Crécy/Poitiers) ; boue en disques sans lien avec l'eau.
5. Abords de siège datés (prairie unie, pas de fossé, faubourg, camp) — capture ancienne.
6. Semis arbres/haies en GDScript à tirages entrelacés : toute retouche change toutes les graines.
7. Composition de la campagne autour des villes non auditée.

## 4. Améliorations
| Prio | Action | Impact | Effort | Dépend de |
|---|---|---|---|---|
| P1 | Terrain et effet sous le curseur (« Verger : couvert +x ») via `RuleValues` | Fort | S | UI, pont |
| P1 | Zones à effet dans la vue tactique et la minimap | Fort | S-M | UI, DA |
| P1 | Sol lisible : boue sombre/brillante, parcelles contrastées, bords nets vus de haut | Fort | M | tech art, DA |
| P2 | Gabarits château/bourg en données (format `landmarks`) | Fort en siège | M | game design |
| P2 | Plan de siège d'Orléans (et Harfleur…) | Moyen-fort | M/ville | historien |
| P2 | Abords de siège (fossé, glacis, faubourgs rasés, lignes, camp) | Moyen-fort | M | sim, env. 3D |
| P2 | « Positions » défensives tirées même en plaine (crête, talus, chemin creux) | Fort | M | game design, IA |
| P3 | Boue et marais liés à l'eau (ADR : graines changées) | Moyen | S-M | sim |
| P3 | Semis arbres/haies en Rust (ADR 0204), changement de graines une fois | Indirect | M | SC |
| P3 | Variantes de cours d'eau | Moyen | M | hydro |
| P3 | Audit des vues de campagne autour des villes emblématiques | Moyen | S puis M | env. carte |

Note : `battle_vegetation.gd`, `settlement_layer.gd`, `landmarks_v2` sont gelés (« GELÉ FL », `docs/wip/sc/restants.md`).
