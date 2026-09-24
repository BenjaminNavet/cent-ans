# WIP — G1 équilibre : sonde (O1), auto-résolution (N1), doctrines (E1)

Branche : `worktree-agent-a41f0303182b63dd5`. Source : `docs/audit/a2-mecaniques.md` § 5.

## État

| Lot | État |
|---|---|
| O1 sonde | en cours : `core/crates/ai/examples/balance_probe.rs` écrit, fixture de calibration `core/crates/ai/tests/fixtures/auto_resolve_scenarios.json` (20 scénarios) |
| N1 auto-résolution | à faire |
| E1 doctrines IA | à faire |
| E8 / E2 | si le temps le permet |

## Sonde (O1)

Depuis `core/` (release conseillé) :

- `cargo run --release -p ai --example balance_probe -- campaign 200 1 2 3 4 5 6 7 8` : parties IA contre IA (défaut 200 tours, graines 1-8).
- `cargo run --release -p ai --example balance_probe -- matrix` : matrice d'auto-résolution (budget, entretien, effectif égaux) et scénarios de calibration.
- `cargo run --release -p ai --example balance_probe -- rt 6` : scénarios de calibration en bataille 3D (6 graines) contre l'auto-résolution ; accord sur le vainqueur. `RT_WRITE=1` réécrit les références 3D de la fixture.
- Sorties : `OUT_DIR` (défaut `core/target/balance_probe/`) reçoit `<mode>.json` et `<mode>.md` ; le Markdown est aussi imprimé.

## Chiffres

(à remplir : avant / après chaque lot)

## Prochaine étape

Lancer `rt` (références 3D) et `campaign` (état initial), commit O1.

## Contraintes

- Ne pas toucher `movement.rs` (M2 en cours ailleurs) sauf point d'appel minimal.
- `ai/src/campaign.rs` : C4 y touche aussi, rester dans des fonctions séparées.
- ADR : numéro 0013 (0011 et 0012 réservés à C4, C5).
