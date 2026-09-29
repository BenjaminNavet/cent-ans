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
- [x] Icônes game-icons + miniatures locales (`tools/cent_ans_tools/unit_emblems.py`, sources `tools/emblem_src/`, `cent-ans assets unit-emblems` puis `entity-icons --build-only`)
- [x] Codex (10 entrées `cdx_*`, catégorie unite)
- [x] Looks de rendu : données + schéma (`data/fx/unit_looks.json`)
- [x] Looks de rendu : `battle_unit_looks.gd` + uniformes `plain_*`/`coat_*` du shader skinné ; script `tests/omr_r5_units_shot.gd`
- [x] Tests core `ai/tests/omr_r5_eastern_units.rs`
- [x] Sonde matrice budget égal : 10 types entre 30 et 80 % (brabançons 16→12 %, archers écossais 88→84 % : hors bande déjà sur main)
- [x] Vérifs finales : fmt, clippy, cargo test --workspace, pytest (1250), import, smoke, `omr_r5_units_shot.gd` headless ; 2 captures (couleurs mamelouks/steppe/droujina lisibles à ~20 m)

## Écarts et points ouverts
- Almogavres : les javelots sont décrits mais pas simulés (infanterie de mêlée) ; pas d'animation
  de lancer à pied.
- Miniatures : emblèmes locaux (silhouette game-icons dorée sur azur), pas des enluminures peintes
  comme les autres unités ; à remplacer si une génération payante est autorisée un jour.
- Imposteurs lointains partagés par (camp, famille, variante) : ils prennent la variante de rendu
  du premier régiment qui les demande.
- Sonde matrice : brabançons (16 % sur main → 12 %) et archers écossais (88 → 84 %) déjà hors bande
  avant R5 ; non retouchés (hors lot).
- Doctrines : `data/ai/doctrines.json` touché (9 factions de l'Est, poids régionaux dans `default`,
  almogavres pour l'Aragon) : conflit possible avec R3 à l'intégration.
- Pas de règle nouvelle dans `core/` (culture/faction/époque existants) : pas d'ADR.

## Prochaine étape
Intégration dans `feat/omr` (lot terminé).
