# WIP — C2 zone de contrôle (rapprochement Total War)

**Suspendu : remplacé par M2 mouvement libre.** Une autre session lance M2
(`docs/design/2026-09-24-mouvement-libre.md`), qui réécrit `movement.rs`
avec sa propre zone de contrôle (8 km, grille A*). Ne rien pousser de plus
sur la ZdC « graphe de colonies » tant que M2 n'est pas fusionné.

Reprendre après la fusion de M2, pour ce que M2 ne couvre pas :
- effets diplomatiques éventuels de la ZdC (aucun commencé) ;
- UI : anneaux atteignables/hachures/infobulle/option « afficher toutes les
  ZdC ennemies » côté `game/` (`reachable_markers.gd`, `campaign_map.gd`) —
  à adapter à la représentation de ZdC que choisit M2 (grille, pas graphe).

## État au moment de l'arrêt

Aucun code écrit : la tâche en était à la phase de lecture/conception
(spec, `movement.rs`, `state.rs`, `data/settlements/rules.json`, schéma
`settlement_rules.schema.json`, `agents.rs` pour confirmer que les agents
ont leur propre Dijkstra hors ZdC, `godot-bridge/src/campaign_sim.rs` et
`game/scripts/map/reachable_markers.gd` pour l'affichage). Rien à committer
côté `core/` ou `game/` : aucun fichier modifié.

## Conception envisagée (non implémentée, peut resservir si M2 ne couvre
## pas tout, à adapter à son modèle de mouvement)

- Nouveau module `core/crates/sim-campaign/src/zone_of_control.rs`,
  `impl CampaignState` :
  - `fn hostile_zoc(&self, data, faction) -> BTreeSet<SettlementId>` :
    sources = colonies où stationne une armée d'une faction en guerre avec
    `faction` d'au moins `min_army_strength`, plus colonies dont la
    garnison (faction en guerre, type dans `garrison_kinds`) atteint
    `min_garrison_strength` ; propagation `radius_hops` sauts sur
    `movement::edges`.
  - Nouvelle section `data/settlements/rules.json` § `zone_of_control`
    (`ZoneOfControlRules` dans `data-model/entities/settlement.rs`,
    `#[serde(default)]` partout, pas de changement de version de
    sauvegarde) : `radius_hops` (déf. 1), `min_army_strength` (déf. 200),
    `min_garrison_strength` (déf. 100), `garrison_kinds` (déf. toutes sauf
    village). Getter `GameData::zone_of_control_rules()` dans
    `movement_graph.rs`, à côté de `movement_rules()`/`retreat_rules()`.
  - `movement::dijkstra` : ajouter un paramètre `respect_zoc: bool` ; bloque
    l'expansion au-delà d'une colonie sous ZdC hostile (comme le blocage
    existant sur colonie/armée hostile), sans empêcher d'y entrer (donc pas
    de traversée, arrêt à l'entrée, sauf attaque = colonie ciblée
    directement). Appels : `reachable`/`find_path` → `true` (les anneaux et
    l'aperçu en tiennent compte automatiquement, sans changement côté
    `game/`) ; `retreat_target` → `false` (le repli garde ses propres
    rayons) ; `ai_minimal.rs` (IA offensive) → `true` (contournement/arrêt
    volontaire obtenus gratuitement par le même Dijkstra).
  - `agents.rs` a son propre `agent_dijkstra` : confirmé non touché.
  - Bridge : `get_hostile_zoc(faction) -> PackedStringArray` (filtré par
    `visible_provinces` pour respecter le brouillard C1) pour les hachures
    et l'option « afficher toutes les ZdC ennemies ».

## Pourquoi suspendu

M2 réécrit `movement.rs` avec un modèle de mouvement libre (grille A*, pas
graphe de colonies) et sa propre ZdC (rayon 8 km). Continuer ce lot créerait
un conflit direct et un travail jeté. Le dossier ci-dessus reste la base de
conception si, après la fusion de M2, il manque les effets diplomatiques ou
l'UI de la ZdC.
