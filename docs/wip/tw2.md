# WIP orchestrateur — TW2 mécaniques Total War (28/09) — REPRIS le 28/09 après-midi

Plan : `docs/design/2026-09-28-tw2-mecaniques-total-war.md`. Mandat : enchaîner les lots sans validation.
Mis en pause par le joueur le 28/09 (« on reprendra dans une future session ») ; rien de TW2 n'est dans main.
ADR : SB 0107, T1 0101, T2 0102, T3 0103, T4 0108, T5 0112 (0100 pris par RS B, 0104-0106 par GA).
Chaque lot a sa note `docs/wip/tw2-<lot>.md` dans sa branche `feat/tw2-<lot>`.

## État

| Lot | Branche | État |
|---|---|---|
| SB barres de vie + rythme siège | feat/tw2-sb (6fcd5080) | **dans main** (befaf258) ; br3 bascule 3/10→10/10 (rééquilibré par T4) |
| T1 sort de la ville prise | feat/tw2-t1 (9851cd5d) | **dans main** (befaf258) |
| T2 reconstitution + réserves | feat/tw2-t2 (fd7e7275) | **dans main** (befaf258) |
| T3 mercenaires | feat/tw2-t3 (e4cdde5e) | **dans main** (c60a6e73) ; suites : surprime absente du panneau budget, illustrations provisoires |
| T4 points de capture + rééquilibrage br3 (cible 4-7/10) | feat/tw2-t4 (597020f1) | agent stoppé pendant sa vérif finale ; capture, dernier carré, repli, UI, tests, ADR 0108 ; voir `docs/wip/tw2-t4.md` |
| T5 traditions d'armée | feat/tw2-t5 (worktree `../gp-tw2-t5`) | agent lancé (inclut dilution d'xp des renforts) |

## integration/tw2 (worktree `../gp-tw2-merge`, 54e3c4b2)

Fusionnée dans main le 28/09 (befaf258) : fmt, clippy, 1116 tests Rust, build, smoke + 3 tests TW2 headless, pytest 880 verts.
NB : pas de `timeout` sur macOS, lancer godot directement.


= main (28/09 matin) + T1 + T2 + SB + correctif souris du test des barres + ADR SB renuméroté 0107.
Vérifié avant la dernière fusion de main : fmt, clippy, cargo test, smoke (29 OK), 3 tests headless TW2,
pytest (seul échec : budget.md de FE, corrigé depuis dans main). **La dernière fusion de main (61 fichiers de
code, dont RS F bataille/feu) n'est pas revérifiée.**

## Reprise
1. integration/tw2 : refusionner main si elle a bougé ; vérif complète (`CARGO_TARGET_DIR=core/target-merge`,
   fmt/clippy/test, `core/build.sh`, `--import`, smoke, `sb_siege_bars_test`, `tw2_t1_capture_test`,
   `tw2_t2_replenish_test`, pytest) ; `git merge --ff-only integration/tw2` depuis main ; push.
2. T4 : agent de reprise sur feat/tw2-t4 (tests complets, br3/sg3 dans ADR 0108), fusion.
3. T3 : agent de reprise sur feat/tw2-t3, fusion.
4. T5 à lancer (inclure la dilution d'expérience des renforts T2).
5. Vérification visuelle (session principale) : barres de siège, fenêtre de capture, drapeaux T4.

## Suites connues
- T1 : griser le recrutement d'une ruine ; marqueur de ruine sur la carte ; fréquence des pillages IA en partie pilote.
- T2 : IA qui se met au repos pour se reconstituer.
- SB : pas d'indicateur « visé » pour béliers/beffrois.
