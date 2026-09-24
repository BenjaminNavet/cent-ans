# Ordres du chef en bataille (lot F10b)

Date : 2026-09-23. Inspiré des capacités du général de Total War (audit UI § 3.2), mais sans magie :
cinq ordres historiques que le chef d'un ost pouvait donner. Règles dans `core/crates/sim-battle`
(`src/orders.rs`), chiffres et textes dans `data/battle_orders/*.json` validés par
`data/schemas/battle_order.schema.json`. Aucun nombre codé en dur.

## 1. Les ordres

| Id | Nom | Libellé par faction | Portée | Recharge | Effet |
|---|---|---|---|---|---|
| `order_war_cry` | Cri de guerre | France « Montjoie ! Saint-Denis ! », Angleterre « Saint George ! », Bourgogne « Montjoie Saint-Andrieu ! », Écosse « Saint Andrew ! », Castille « ¡Santiago! », autres « Cri de guerre » | 150 m autour du chef | 120 s | +12 moral, et +12 au plafond de récupération pendant 30 s ; le cri paraît au journal. |
| `order_no_quarter` | Pas de quartier | France « Déployer l'oriflamme », Angleterre « Lever la bannière au dragon », autres « Déployer la bannière rouge » | toute l'armée | 1 fois par bataille | +20 moral et +20 au plafond pour la bataille ; `no_quarter: true` dans le `SideResult` du camp. |
| `order_dismount` | Pied à terre | — | régiments désignés (tous les éligibles si aucun) | 5 s | Cavalerie lourde avec la capacité `dismount` : infanterie, vitesse ≤ 35, armure +8, plus de charge, coin → ligne. Irréversible. |
| `order_pavise` | Dresser les pavois | — | régiments désignés (capacité `pavise`) | 10 s | Pertes par le trait × 0,35 (au lieu de × 0,6 passif), régiment immobile ; levé au prochain ordre de mouvement (déplacement, retraite, attaque hors de portée de tir) ou à la déroute. |
| `order_rally` | Rallier | — | régiments en déroute à 200 m du chef | 90 s | Chaque régiment revient (état « rallié », moral ≥ 45) avec la chance 0,25 + 0,06 × commandement ; échec au journal. |

Références historiques dans les descriptions : oriflamme de Saint-Denis (Crécy, Poitiers), bannière au
dragon d'Édouard III (Crécy 1346), pied à terre anglais (Crécy) puis français (Poitiers 1356), pavois
génois restés aux bagages à Crécy, Du Guesclin à Cocherel (1364). Les pieux des archers restent
automatiques (M7) et ne sont pas un ordre.

Les ordres `requires_general: true` (cri, pas de quartier, rallier) exigent un chef vivant, sur le champ
et pas en déroute. Refus en français (`CommandError::OrderUnavailable { order, reason }`) : « aucun chef
à la tête de l'armée », « le chef est tombé », « le chef ne commande plus », « déjà donné dans cette
bataille », « recharge, encore N s », « aucun régiment autour du chef », « aucune cavalerie lourde à
démonter », « aucun arbalétrier à pavois disponible », « aucun régiment en déroute près du chef »,
« aucun des régiments désignés ne peut l'exécuter ». Ordre inconnu : `UnknownOrder`, ordre pour l'autre
camp : `WrongSide`.

## 2. Données (`battle_order.schema.json`)

`id`, `kind` (`war_cry | no_quarter | dismount | pavise | rally`), `rank` (place dans la barre), `name`,
`label` (libellé par défaut), `labels_by_faction`, `description`, `icon` (facultatif,
`game/assets/ui/orders/<icon>.png`, glyphe de repli sinon), `scope` (`radius | army | selected`),
`requires_general`, `cooldown`, `duration`, `radius`, `uses_per_battle`, `eligible` (catégories, monté,
capacités), `effects` (`morale`, `speed_max`, `armor`, `missile_damage_factor`, `rally_chance`,
`rally_chance_per_command`, `rally_morale`), `journal` (jokers `{label}`, `{faction}`, `{of_faction}`,
`{general}`, `{unit}`, `{count}`), `journal_failure`, `journal_assault`, `ai`, `sources`.

Chargement : `GameData::battle_orders` (dossier facultatif) ; `CampaignState::battle_setup` recopie le
catalogue dans `BattleSetup::orders` (`#[serde(default)]`, rétrocompatible : sans catalogue, aucun ordre).

## 3. Simulation

- `Command::LeaderOrder { side?, order, units }` (`{"type": "leader_order", ...}`). Le camp est celui
  qui donne l'ordre (camp du joueur ou de l'IA), sinon `side`, sinon celui du premier régiment.
- Usages et recharges par camp dans `BattleSim` (`order_use`), en temps simulé : déterministe (le
  ralliement tire dans le `BattleRng` de la bataille). Mêmes graines et mêmes ordres aux mêmes ticks →
  même bataille (test).
- `BattleSim::leader_orders(side) -> Vec<OrderView>` : id, nature, nom, libellé de la faction,
  description, icône, disponible, raison, recharge et recharge restante, usages.
- Pied à terre factorisé : `Unit::dismount(speed_max, armor)` sert aussi au démontage automatique des
  chevaliers avant un assaut de siège (M8), dont la ligne de journal vient de `journal_assault`
  (« Les chevaliers de France mettent pied à terre pour l'assaut. »), sans le bonus d'armure.
- Pas de quartier : si le camp vainqueur l'a déployé, un chef vaincu rattrapé en déroute est tué au lieu
  d'être pris. Le drapeau `SideResult::no_quarter` est transmis à la campagne, qui l'exploite (P1,
  `resolve_pending_battle`) : si le **vainqueur** a donné l'ordre, le chef vaincu signalé pris est
  tué (aucun prisonnier, donc aucune rançon H6), et le chef vainqueur perd `NO_QUARTER_PIETY` (5)
  de piété ; une ligne « Pas de quartier : … » paraît au journal. L'ordre du vaincu n'a pas d'effet
  en campagne.

## 4. IA tactique (`ai.rs`, `plan_orders`)

Conditions du bloc `ai` de chaque ordre, évaluées toutes les 2 s après le plan de mouvement :
cri de guerre quand un ennemi est à moins de 90 m d'au moins deux régiments autour du chef ; rallier dès
qu'un régiment fuit près du chef ; pied à terre en posture défensive (et pour la garnison d'un siège) ;
pavois pour les arbalétriers à l'arrêt touchés par le trait dans les 6 dernières secondes ; pas de
quartier seulement pour une armée en infériorité (rapport de forces < 0,75), à moins de 150 m de
l'ennemi héréditaire (France–Angleterre, Écosse–Angleterre).

## 5. Interface

- Pont : `BattleSim.issue_command({type: "leader_order", order, units})`,
  `BattleSim.get_leader_orders(side) -> Array[Dictionary]`, `get_no_quarter(side)` ; `get_units`
  expose `pavise`, `dismounted`, `order_morale`.
- `game/scripts/battle/leader_orders_bar.gd` : rangée de boutons « Ordres du chef » en bas à gauche,
  au-dessus des cartes d'unités (calque 2, sans toucher `battle_hud.gd`) ; glyphe, nom, raccourci ;
  grisé si indisponible ; rideau sombre et secondes restantes pendant la recharge ; infobulle riche (nom,
  libellé de la faction, description historique, recharge, raison du refus). Raccourcis Z, X, V, B, N
  dans l'ordre de la barre (1-3 vitesse, F/G/H ordres, WASD/Q/E caméra, C/M/T carte déjà pris). Les
  ordres « sélection » visent la sélection ; si aucun régiment choisi ne peut obéir, tous les régiments
  concernés.
- Branchement : trois lignes dans `battle_scene.gd` (constante `LEADER_ORDERS_BAR`, `add_child` à la
  fin de `begin`).
- Capture : `docs/img/battle-leader-orders.png`.

## 6. Tests

`core/crates/sim-battle/tests/orders.rs` (18) : catalogue et libellés, sérialisation de la commande,
cri (rayon, expiration, recharge), refus (sans chef, chef en déroute, id inconnu, mauvais camp), pas de
quartier (armée entière, une fois, drapeau dans le résultat), pied à terre (effet, sélection,
irréversible, siège), pavois (pertes réduites, levés par un déplacement), ralliement (chance,
commandement, journal), déterminisme, vue de la barre, IA (cri à l'engagement, ralliement, pavois sous
le tir, pied à terre en défense, pas de quartier seulement contre l'ennemi héréditaire, bataille
complète déterministe). `tools/tests/test_battle_orders_schema.py` valide les JSON. Le smoke Godot
vérifie `get_leader_orders` et l'ordre de cri.
