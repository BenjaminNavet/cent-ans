# WIP — G1 équilibre : sonde (O1), auto-résolution (N1), doctrines (E1)

Branche : `worktree-agent-a41f0303182b63dd5`. Source : `docs/audit/a2-mecaniques.md` § 5.

## État

| Lot | État |
|---|---|
| O1 sonde | **fait** : `core/crates/ai/examples/balance_probe.rs`, fixture de calibration `core/crates/ai/tests/fixtures/auto_resolve_scenarios.json` (20 scénarios, références 3D sur 6 graines) |
| N1 auto-résolution | **fait** : `battle_auto.rs` par phases, `data/rules/auto_resolve.json` + schéma, test `ai/tests/auto_resolve_calibration.rs` (20/20), ADR 0013 ; `movement.rs` : seul l'appel `resolve_field` (et l'import) change |
| E1 doctrines IA | **fait** : `data/ai/doctrines.json` + `ai_doctrine.schema.json`, `ai/src/doctrine.rs` (`pick_recruit`, `field_composition`), boucle de recrutement de `plan_economy` modifiée localement, tests `ai/tests/e1_doctrines.rs` |
| E8 coûts d'unités | **fait** : coûts et entretiens de 8 types dans `data/unit_types/` |
| E2 ordre public | **fait (partiel)** : `data/rules/population.json` + `population_rules.schema.json` ; `population.rs` lit ces termes (poids de l'impôt 40 → 100, soulagement par la garnison plafonné à 10, biens plafonnés à −10, occupation 25 → 35) ; ni fatigue de guerre ni changement des seuils d'impôt de l'IA |

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

### Après E1

- `campaign 200 1..8` : milice **37,4 %** des recrutements (avant 99,8 %), archers longs **50 %** des recrutements anglais (avant 0 %), **8 types** recrutés par partie au minimum (avant 1) : les trois cibles E1 sont tenues.
- Effets de bord : les unités recrutées sont plus chères, donc les armées plus petites (France 3 000 à 12 000 hommes en campagne en 1387 au lieu de 13 000 à 34 000) : 136 batailles par partie (avant 334), 5,4 changements de propriétaire (avant 12). Guerre FR-EN sur 200 tours : 40 %.
- `century_probe 464 1..5` (doctrine milice 30) : guerre FR-EN 56, 36, 28, 47, 54 % (moyenne 44 %, avant 40 %) ; majeures en 1400 : 5/5. Pas de régression.

### Après E8 (coûts d'unités)

| Unité | Coût / entretien avant | Après |
|---|---|---|
| Arbalétriers | 550 / 50 | 450 / 40 |
| Piquiers flamands | 450 / 40 | 600 / 55 |
| Arbalétriers génois | 900 / 100 | 600 / 55 |
| Chevaliers | 1 500 / 140 | 1 400 / 130 |
| Archers longs | 500 / 45 | 650 / 60 |
| Hommes d'armes à pied | 900 / 90 | 950 / 90 |
| Archers montés | 700 / 60 | 600 / 55 |
| Sergents à cheval | 800 / 80 | 750 / 70 |
| Milice urbaine | 300 / 25 | inchangé |

- Matrice à budget égal (victoires moyennes) : arbalétriers 53 %, piquiers 75 %, génois 39 %, chevaliers 76 %, archers longs 50 %, hommes d'armes 75 %, archers montés 26 %, sergents 31 %, milice 28 % : toutes entre 20 et 80 % (cible E8 ; avant : de 6 à 95 %).
- `campaign 200 1..8` : milice 36 %, archers longs 44,5 % des recrutements anglais, 8 types au minimum ; guerre FR-EN 41 % ; 139 batailles ; 6,1 changements de propriétaire.
- `century_probe 464 1..5` : guerre FR-EN 54, 49, 33, 51, 43 % (moyenne 46 %, avant G1 40 %) ; majeures en 1400 : 5/5.

### Après E2 (ordre public)

- Mesure corrigée dans la sonde : le mécontentement moyen est désormais celui des classes (`weighted_unrest`), et non le champ `ProvinceState::unrest` (toujours proche de 0). Avant E2 : 3,4.
- Poids de l'impôt 70 : mécontentement moyen 3,4, 5,1 révoltes par partie. Poids 100 (retenu) : mécontentement moyen **8,0** (cible 15-35 non atteinte), **15,5 révoltes** par partie de 200 tours (avant 1), impôt « Haut » dans 47 % des échantillons (cible < 40 % non atteinte : l'IA garde son seuil de 30).
- `campaign 200 1..8` : milice 36 %, 9 types, guerre FR-EN 44 %.
- `century_probe 464 1..5` : guerre FR-EN 57, 41, 56, 46, 37 % (moyenne 47 %) ; majeures en 1400 : 5/5.

### Après fusion de main (M2-M4, C4, C5, U1, CV1...)

- Conflits : `movement.rs` (appel `resolve_field` rebranché dans le nouveau `auto_fight` de M2) ; déclarations `ai_grid` (M3) et doctrines/règles côte à côte dans `data-model` et `ai/src/lib.rs`. La boucle de recrutement par doctrines a fusionné sans conflit dans le `campaign.rs` de C4.
- Sièges : assauts et sorties passent par `battle_auto::resolve_profiled` (profils de la garnison via `army_profiles`) ; l'assaut garde les murailles sans terrain, la sortie se bat sur le terrain de la province.
- IA : impôt « Haut » seulement sous un mécontentement de 18 (`HIGH_TAX_MAX_UNREST`, avant 30).
- Référence main (70c9e2e5), `century_probe 464 1..5` : guerre FR-EN 39, 19, 34, 52, 46 % (moyenne 38 %), batailles FR/EN 43 à 113 par décennie, majeures 5/5.
- G1 fusionné, `century_probe 464 1..5` : guerre FR-EN 46, 45, 41, 33, 32 % (moyenne **39 %**), batailles FR/EN 42 à 91 par décennie, majeures **5/5** : pas de régression par rapport à main. La cible 55-75 % n'est pas tenue, ni sur main ni ici.
- G1 fusionné, `campaign 200 1..8` : milice 34 %, archers longs 51 % des recrutements anglais, 9 types au minimum, mécontentement moyen 7,2, impôt « Haut » 38 % des échantillons (cible E2 < 40 % désormais tenue), 7,2 révoltes par partie, 153 batailles.
- `rt 0` : 20 / 20. `cargo test` : 67 lots ok. Smoke Godot : OK (24 lignes OK).
- Note : sur le main intermédiaire fusionné d'abord (23f9e0a9^2), les batailles FR/EN tombaient à 3-5 par décennie, avec ou sans G1 (mesuré par variantes) ; corrigé par main depuis.

## Prochaine étape

Hors G1 : guerre FR-EN sous la cible (main compris) ; fatigue de guerre (N2) ; cible de mécontentement encore basse (7 contre 15-35) ; la sonde compte mal les victoires de l'attaquant depuis M2 si le texte des batailles change encore.

## Contraintes

- Ne pas toucher `movement.rs` (M2 en cours ailleurs) sauf point d'appel minimal.
- `ai/src/campaign.rs` : C4 y touche aussi, rester dans des fonctions séparées.
- ADR : numéro 0013 (0011 et 0012 réservés à C4, C5).
