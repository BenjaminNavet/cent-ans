# ADR 0002 — Un fichier par jalon dans le pont et l'interface

Date : 2026-09-23. Statut : accepté.

## Contexte

Les jalons M5 à M10 ont été développés en parallèle par plusieurs agents, chacun dans son worktree git.
Les fichiers partagés (`campaign_sim.rs`, `campaign_map.gd`, `campaign_map.tscn`) concentraient les
conflits de fusion.

## Décision

- Pont GDExtension : chaque jalon ajoute ses méthodes à `CampaignSim` dans un bloc
  `#[godot_api(secondary)]` de son propre fichier (`campaign_sim_diplomacy.rs`, `campaign_sim_tech.rs`,
  `campaign_sim_siege.rs`, `campaign_sim_victory.rs`, `battle_sim.rs`…). `campaign_sim.rs` n'expose que
  `data`/`state` et quelques helpers en `pub(crate)`.
- Interface Godot : chaque jalon ajoute un contrôleur (`DiplomacyController`, `SiegeController`,
  `VictoryController`, `HelpController`…) qui construit son UI en code et s'accroche à `campaign_map.gd`
  par deux à quatre appels (`setup`, `refresh`, `after_end_turn`, `handle_input`), sans modifier les scènes.
- Simulation : la logique d'un jalon vit dans ses modules (`diplomacy.rs`, `religion.rs`, `research.rs`,
  `battle_request.rs`, `victory.rs`…), avec des champs d'état `#[serde(default)]` et une seule valeur de
  `STATE_VERSION` partagée (4).
- Les scripts compilés par le smoke test (`--script`) n'utilisent pas les identifiants d'autoload à la
  compilation : `get_node("/root/SimFacade")` à l'exécution.

## Conséquences

Fusions presque mécaniques (ajouts côte à côte). En contrepartie, l'UI construite en code est moins
visible dans l'éditeur Godot, et `campaign_map.gd` accumule de petits points d'accroche.
