# Plan — seed bots (bots de test à graine sur `core/`)

Spec : `docs/superpowers/specs/2026-10-13-seed-bots-design.md`. Note d'avancement :
`docs/wip/seed-bots.md`. Worktree `../game_project-seed-bots`, branche `seed-bots`, fusion
`--ff-only` dans le `main` local à la fin. **Aucun push sans feu vert. Aucune passe longue sans feu
vert du joueur.**

Socle existant à réutiliser (ne pas réécrire) :
- `CampaignState::new_1337(data, player, seed)`, `GameData::load(&root)` ;
- `end_turn_with(data, ai::plan_turn)` pour les factions IA et le bot miroir ;
- `crates/ai/examples/playthrough.rs` : déjà une partie complète IA contre IA (modèle du runner) ;
- `submit_order` / `submit_order_outcome` (`orders/mod.rs`), `save_json` / `load_json` (`save.rs`),
  `agents::derived_rng`, prérequis dans `preview.rs`.

Profil de build : `[profile.seed-bots]` dans `core/Cargo.toml`, `inherits = "release"`,
`lto = false`, `codegen-units = 16` (le profil `release` en LTO + 1 unité coûte ~10 min de build,
inacceptable pour une passe de 30 s en CI).

## Lot 0 — squelette (session principale)

1. Worktree + branche `seed-bots`.
2. Crate `core/crates/seed-bots` ajouté au workspace : `lib.rs`, `bot.rs` (trait `Bot`),
   `action.rs` (`enum Action`), `view.rs` (`BotView`), `bots/mod.rs`, `oracles/mod.rs`
   (`Finding`, `Severity`, `OracleContext`), `runner.rs`, `report.rs`, `main.rs` (aide seulement).
   Signatures publiques complètes, corps `todo!()`, tests `#[ignore]`.
3. `sim-campaign/src/invariants.rs` : `pub fn check_invariants(&CampaignState, &GameData) ->
   Vec<Violation>` qui renvoie `vec![]`.
4. Profil `seed-bots`. Note wip. `cargo build`, `clippy`, commit `wip(seed-bots): skeleton`.

## Vague 1 — trois agents en parallèle

### Lot A — invariants et oracles 1-3 (`cent-ans-dev`)
- `invariants.rs` : les 6 invariants de la spec, chacun avec un test (état valide passe, état
  corrompu à la main est signalé).
- `oracles/panic.rs` (enveloppe `catch_unwind`, mesure du tour, limite configurable),
  `oracles/save_roundtrip.rs`, `oracles/invariants.rs`, `oracles/refused.rs`.
- Tests unitaires < 1 s au total.

### Lot B — vue, actions, génération d'actions légales, bots random-legal et miroir (`cent-ans-dev`)
- `BotView` sur les requêtes déjà exposées à l'interface ; `Action` couvre `Order` et les actions
  hors `Order` (diplomatie, recherche, édits, réponses aux événements) via l'API publique existante.
- `candidates.rs` : liste des actions que les prérequis/prévisualisation déclarent possibles.
- `bots/random_legal.rs`, `bots/mirror.rs` (délègue à `ai::plan_turn`).
- Si une famille d'actions n'a pas d'API de prérequis, la lister dans la note wip (pas de contournement).

### Lot C — rapports, CLI, bandes (`cent-ans-mech`, Sonnet)
- `report.rs` : `report.json` (serde), `report.md` en français, déduplication par
  (oracle, signature), compteur d'occurrences.
- `main.rs` : `run [--quick] [--seeds a..b] [--turns n] [--bots liste] [--max-minutes m]`,
  `replay <constat.json>`.
- `data/balance/seed_bots_bands.yaml` (bandes vides marquées « à initialiser ») +
  `data/schemas/seed_bots_bands.schema.json` + validation au chargement.

## Vague 2 — deux agents

### Lot D — runner et rejeu (`cent-ans-dev`)
- Un fil par bot ; boucle plan → soumission → oracles → `end_turn_with` → oracles.
- Aller-retour de sauvegarde tous les 10 tours et en fin ; plafond dur (`--max-minutes`, 15 par
  défaut) avec rapport partiel ; journal d'ordres par partie.
- `replay` rejoue à l'identique et s'arrête au tour fautif.
- Tests : déterminisme (deux passes rapides de même graine → `report.json` identique) ; constat
  provoqué (invariant cassé par un drapeau de test) rejoué au même tour.
- **Mesurer** la passe rapide et une passe longue réduite (1 graine × 150 tours × 8 bots) ;
  ajuster graines/tours par défaut pour tenir ≤ 30 s et ≤ 15 min ; noter les chiffres dans la note wip.

### Lot E — les 6 autres bots (`cent-ans-mech`, Sonnet)
- `aggressor`, `builder`, `diplomat`, `turtle`, `sailor`, `chaos` : chacun filtre/pondère
  `candidates.rs` selon la spec ; un test par bot (sur 5 tours, il émet bien des actions de sa
  famille).

## Vague 3

### Lot F — dérive d'équilibre (`cent-ans-dev`)
- `metrics.rs` : révoltes, faillites, durée des guerres, factions éliminées, part de territoire de
  la première faction ; agrégation par bot sur N graines.
- `oracles/balance.rs` : comparaison aux bandes, gravité `avertissement`.
- **Arrêt : proposer au joueur la passe longue de référence** (durée estimée) ; après feu vert,
  la lancer en arrière-plan et initialiser les bandes à partir de son résultat.

### Lot G — intégration (`cent-ans-mech`, Sonnet)
- CI (`.github/workflows/windows.yml` ou nouveau job) : `cargo run --profile seed-bots -p seed-bots
  -- run --quick`.
- `CLAUDE.md` (Commandes + règle : passe rapide en fin de lot ; passe longue proposée au joueur
  après une grosse fonctionnalité, jamais lancée d'office ; constats validés par le joueur avant le
  backlog `docs/wip/seed-bots-findings.md`).
- `docs/architecture.md` : le crate et son rôle.
- ADR 0347 : crate, oracles, niveaux d'exécution, validation par le joueur.

## Clôture (session principale)
- Relecture du diff complet, `cargo fmt`, `clippy -D warnings`, `cargo test`, passe rapide.
- Fusion `--ff-only` dans `main`, suppression du worktree, mémoire mise à jour.
