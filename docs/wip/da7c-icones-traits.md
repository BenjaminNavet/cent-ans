# DA7c — une icône pour chaque trait de personnage

Bible `docs/design/2026-09-25-bible-da.md` § 8, ADR `docs/decisions/0065-boutons-et-icones-enlumines.md`
(§ DA5b conclut : « des miniatures par trait (59) sont un lot possible »). Plafond de dépense
**4 $** pour ce lot (distinct des 5 $ de DA5, déjà consommés à 4,84 $).

## État

1. Inventaire fait : les 59 traits (`data/traits/*.json`) n'avaient pas d'icône propre ; les
   4 points d'affichage (`RichTooltip.trait_tip`, `character_sheet._fill_traits`,
   `Encyclopedia._entry_icon`, `Encyclopedia._trait_fiche`) utilisent tous
   `"trait_category_" + category` (icône de catégorie générique, 5 valeurs). La table de
   correspondance id → icône du jeu est `data/ui/icons_ink.json` (schéma
   `data/schemas/icons_ink.schema.json`), lue par `IconLibrary` (`game/scripts/ui/icon_library.gd`)
   via `game/assets/icons/ink/index.json`.
2. **Données** : 59 entrées ajoutées à `data/ui/icons_ink.json["icons"]`, `"group": "trait"`
   (ajouté à l'enum du schéma), `id == targets[0] == <id du trait>` (ex. `trait_admiral`) —
   donc `IconLibrary.get_icon("trait_admiral")` prend l'icône directement, sans passer par une
   catégorie. Un prompt visuel par trait (anglais, sans texte), même bloc de style DA5
   (`icon_style` du catalogue, inchangé).
3. **Outil réutilisé** (`tools/cent_ans_tools/ink_icons.py`, `cent-ans assets ink-icons`), pas de
   nouvel outil : `Entry` gagne un champ `group`, `entries()`/`plan()` un filtre `group=`,
   `lot_spent()` un paramètre `prefix` (défaut `"DA5 :"`, rétro-compatible). La commande CLI
   gagne `--group`, `--subject`, `--budget-cap` pour qu'un autre lot (ici DA7c) ait sa propre
   ligne de grand livre et sa propre enveloppe sans toucher au plafond DA5 déjà quasi épuisé
   (0,16 $ restants sur ses 5 $). Invocation prévue :
   ```
   uv run --project tools cent-ans assets ink-icons --group trait \
     --subject "DA7c : icônes de trait à l'encre" --budget-cap 4.0 --dry-run
   ```
   puis sans `--dry-run` pour lancer réellement (mêmes options).
4. Pytest `tools/tests/test_ink_icons.py` (8 tests, dont le schéma) passent avec les 59 entrées.

## Prochaine étape

- Lancer `--dry-run` pour le coût estimé (~59 × 0,0455 $ ≈ 2,68 $, sous le plafond de 4 $).
- Lancer la génération réelle (guet-apens du garde-fou de permissions possible, comme pour DA2 :
  si bloqué, tout commiter et donner la commande exacte dans le rapport).
- `cent-ans assets ink-icons --group trait --build-only` pour dériver les PNG + `ink/index.json`.
- Godot : `CARGO_TARGET_DIR=$PWD/core/target core/build.sh` puis
  `godot --headless --path game --import`.
- GDScript : remplacer `"trait_category_" + category` par l'id du trait (repli catégorie
  déjà géré par `IconLibrary.resolve`) dans les 4 sites listés au point 1.
- pytest `data/traits` : chaque trait a une entrée `icons_ink.json` (et son PNG après build).
- Test Godot si pertinent (smoke ou test dédié `da7c`).
- `docs/budget.md` (dépense réelle), ADR 0065 § DA7c, capture `docs/img/da7c/`.
