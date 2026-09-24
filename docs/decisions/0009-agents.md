# ADR 0009 — Agents de campagne : entités légères hors dynastie

Date : 2026-09-24. Statut : accepté. Lot C6 (`docs/design/2026-09-24-agents.md`).

## Contexte

L'analyse Total War (T9) proposait de réutiliser les personnages M4 comme agents. Or `characters.rs` et la
fiche de général sont retravaillés en parallèle (lot C7), `ProvinceState`/`SettlementState` et
`movement.rs` le sont par la refonte des colonies (C7a), et la sauvegarde v5 vient d'être figée.

## Décision

- Les agents sont des entités propres (`sim-campaign/src/agents.rs`) : `AgentsState` dans
  `CampaignState::agents`, en `#[serde(default)]`, sans changement de `STATE_VERSION`.
- Déplacement sur le graphe des colonies via les fonctions publiques de `movement.rs` (`edges`), avec
  un Dijkstra propre qui ne s'arrête ni aux colonies ennemies ni aux armées.
- Tirages déterministes par un générateur dérivé de (graine, tour, agent, action), sans consommer
  `CampaignState::rng` : les parties sans agents gardent exactement les mêmes tirages.
- Ordres ajoutés à `Order` (`recruit_agent`, `move_agent`, `agent_action`, `dismiss_agent`) pour passer
  par la même validation que le joueur ; l'IA des agents (`agents::plan_agents`) vit dans `sim-campaign`
  comme la diplomatie et la recherche (ADR 0003) et n'est appelée que par le planificateur stratégique
  `ai::plan_turn` ; le planificateur minimal ne recrute pas d'agents, ce qui garde les tests M2-M10 stables.
- Règles chiffrées dans `data/rules/agents.json` (schéma `agent_rules.schema.json`) ; valeurs par défaut
  codées identiques pour les tests sans données.

## Conséquences

- Les agents ne vieillissent pas, ne se marient pas, n'héritent pas ; une future fusion avec M4 (prélats
  titrés, hérauts nommés) reste possible en ajoutant un lien optionnel vers un `CharacterId`.
- Le fichier de données s'écarte de la consigne `data/agents.json` pour suivre la convention des règles
  globales (`data/rules/vision.json`, `siege_fire.json`).
