# OMR R5 — unités propres à l'Est (1337)

Branche `feat/omr-r5`, worktree `../gp-omr-r5`. Plan d'ensemble : `docs/wip/omr.md`.

## Décisions
- Recrutement : règles existantes seules (culture de province ET faction, époque) ; pas de règle
  nouvelle dans `core/`, donc pas d'ADR.
- `unit_mounted_archers` (arc long anglais, combat à pied, tech. `tech_longbow_drill`) n'est pas
  étendu : les archers des steppes tirent à cheval à l'arc composite. Nouveau type distinct,
  même figurine de tireur monté (`cavalry_2`, style `horse_bow`).
- Rendu : figurines existantes ; variantes (étoffes, robes de chevaux, part de livrée) dans
  `data/fx/unit_looks.json` (rendu seulement).
- Icônes : game-icons.net (CC BY 3.0) via `icons_catalog.py`, sans dépense.

## Unités
| Id | Nom | Accès | Figurine |
|---|---|---|---|
| unit_mamluk_cavalry | Mamelouks | fac_mamluks | cavalry_2 |
| unit_steppe_horse_archers | Archers montés des steppes | cultures kiptchak, bachkire, turque | cavalry_2 |
| unit_akinci | Akıncı | fac_ottoman | cavalry_6 |
| unit_yaya | Yaya | fac_ottoman, culture turque, 1330-1450 | archer_3 |
| unit_serbian_heavy_cavalry | Cavalerie lourde serbe | culture serbe | cavalry_0 |
| unit_pronoiars | Pronoïaires | Byzance, Trébizonde, Épire ; culture grecque | cavalry_4 |
| unit_druzhina | Droujina | cultures russe, novgorodienne, ruthène | cavalry_1 |
| unit_lithuanian_light_cavalry | Cavalerie légère lituanienne | culture lituanienne | cavalry_5 |
| unit_teutonic_knights | Frères chevaliers teutoniques | Ordre teutonique, Ordre livonien | cavalry_0 |
| unit_almogavars | Almogavres | culture catalane, jusqu'en 1400 | infantry_3 |

## État
- [x] Données des 10 types (schéma valide)
- [x] Doctrines IA (9 factions + poids régionaux dans `default`), éclaireurs, gore, étendards
- [x] Icônes game-icons
- [ ] Codex
- [ ] Looks de rendu (données + shader)
- [x] Tests core `ai/tests/omr_r5_eastern_units.rs`
- [ ] Sonde matrice (20-80 %)
- [ ] Vérifs finales (fmt, clippy, tests, pytest, import, smoke), 1 capture

## Prochaine étape
Codex, looks de rendu, sonde matrice.
