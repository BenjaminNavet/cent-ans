# JR4 — IA et équilibrage de la faction croisée

Spec `docs/superpowers/specs/2026-10-02-jr-croises-jerusalem-design.md`, note d'orchestration
`docs/wip/jr-croises.md`. Worktree `../gp-jr` (`feat/jr`), en parallèle de JR3 (`game/`).

## État
- Sonde `core/crates/ai/tests/jr_crusade_probe.rs` (`#[ignore]`, 5 graines × 50 tours, ~50 s) :
  `cargo test -p ai --test jr_crusade_probe -- --ignored --nocapture` ; `JR_SEEDS=1,4`,
  `JR_TURNS=60`, `JR_TRACE=1`.
- Référence avant correctif (barème JR1) : graines 1, 4, 5 → ferveur 0 dès le tour 30, 2-3 unités,
  aucune place ; graines 2, 3 → ferveur 99, 6-7 places tenues. Débarquement au tour 1 partout.

- Blocage IA trouvé : au tour 1 l'IA croisée achetait la paix aux Mamelouks avec tout son trésor
  (8 000 livres), puis licenciait faute d'argent ; en paix, −3/tour → ferveur 0. Correctif :
  `crusade::ai_vow_forbids_peace` (la faction des règles, menée par l'IA, ne propose ni n'accepte
  la paix avec le maître de la province cible) dans `negotiation` (blocage + `plan_peace`) et
  `diplomacy` (`evaluate` + ancien chemin). Ensuite l'IA débarque dès le tour 1 et prend des places.
- Règle : malus « frères de foi » seulement si les croisés attaquent (`on_battle(..., attacker)`),
  crochet naval dans `naval::apply_outcome`, `WORLD_NEWS_MARK`/`is_world_news` (délivrance et
  perte de la cible lues par tous). Tests d'intégration par l'ordre `Attack` réel.

## Prochaine étape
- Barème : premier passage (usure 2, victoire 3, place 4, prêche 8, passage 1 unité/40 max 3,
  recharge 8) → Jérusalem prise 3/5, une graine en spirale vers 0 ; réserve IA pour le passage.

## Points ouverts
- (à compléter)
