# WIP — G1 équilibre : sonde (O1), auto-résolution (N1), doctrines (E1)

Branche : `worktree-agent-a41f0303182b63dd5`. Source : `docs/audit/a2-mecaniques.md` § 5.

## État

| Lot | État |
|---|---|
| O1 sonde | **fait** : `core/crates/ai/examples/balance_probe.rs`, fixture de calibration `core/crates/ai/tests/fixtures/auto_resolve_scenarios.json` (20 scénarios, références 3D sur 6 graines) |
| N1 auto-résolution | **fait** : `battle_auto.rs` par phases, `data/rules/auto_resolve.json` + schéma, test `ai/tests/auto_resolve_calibration.rs` (20/20), ADR 0013 ; `movement.rs` : seul l'appel `resolve_field` (et l'import) change |
| E1 doctrines IA | à faire |
| E8 / E2 | si le temps le permet |

## Sonde (O1)

Depuis `core/` (release conseillé) :

- `cargo run --release -p ai --example balance_probe -- campaign 200 1 2 3 4 5 6 7 8` : parties IA contre IA (défaut 200 tours, graines 1-8).
- `cargo run --release -p ai --example balance_probe -- matrix` : matrice d'auto-résolution (budget, entretien, effectif égaux) et scénarios de calibration.
- `cargo run --release -p ai --example balance_probe -- rt 6` : scénarios de calibration en bataille 3D (6 graines) contre l'auto-résolution ; accord sur le vainqueur. `RT_WRITE=1` réécrit les références 3D de la fixture.
- Sorties : `OUT_DIR` (défaut `core/target/balance_probe/`) reçoit `<mode>.json` et `<mode>.md` ; le Markdown est aussi imprimé.

## Chiffres

### Avant (main à 15363eb + M1, auto-résolution d'origine)

Sonde `campaign 200 1..8` :

| Mesure | Avant |
|---|---|
| Milice / recrutements | 99,8 % |
| Archers longs / recrutements anglais | 0 % (0 / 842) |
| Types d'unités recrutés par partie (min) | 1 (max 3) |
| Guerre France-Angleterre (200 tours) | 42 % |
| Changements de propriétaire | 12 |
| Batailles par partie (attaquant gagne) | 334 (43 %) |
| Bâtiments jamais construits | 10 / 29 |

Sonde `rt 6` (20 scénarios) : accord auto / 3D sur le vainqueur **12 / 20 (60 %)**. Désaccords : chevaliers contre milice (dans les deux sens), hommes d'armes contre milice (dans les deux sens), sergents contre arbalétriers, chevaliers contre hommes d'armes, piquiers contre hommes d'armes, cavalerie contre milice en forêt.

`century_probe 464 1..5` : guerre France-Angleterre 43, 34, 48, 34, 39 % (moyenne 40 %, **déjà sous la cible 55-75 % avant G1** : régression antérieure, cf. audit § 6) ; 4 majeures vivantes en 1400 : 5/5.

### Après N1

- `rt 0` : accord auto / 3D **20 / 20 (100 %)** (avant 12 / 20).
- Matrice à budget égal (victoires moyennes) : arcs longs 95 %, hommes d'armes 81 %, piquiers 76 %, chevaliers 61 %, milice 50 %, sergents 39 %, arbalétriers 27 %, archers montés 19 %, génois 6 % (à corriger par E8).
- `campaign 200 1..8` : milice 99,9 % (l'IA ne change pas encore de recrutement : E1), guerre FR-EN 44 %, 14,4 changements de propriétaire, 316 batailles (l'attaquant gagne 40 %).
- `century_probe 464 1..5` : guerre FR-EN 40, 47, 39, 39, 40 % (moyenne 41 %, avant 40 %) ; majeures en 1400 : 5/5. Pas de régression.

## Prochaine étape

E1 : `data/ai/doctrines.json` + schéma, module `ai/src/doctrine.rs`, appel localisé dans `plan_economy`.

## Contraintes

- Ne pas toucher `movement.rs` (M2 en cours ailleurs) sauf point d'appel minimal.
- `ai/src/campaign.rs` : C4 y touche aussi, rester dans des fonctions séparées.
- ADR : numéro 0013 (0011 et 0012 réservés à C4, C5).
