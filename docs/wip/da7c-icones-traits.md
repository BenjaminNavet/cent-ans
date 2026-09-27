# DA7c — une icône pour chaque trait de personnage

Bible `docs/design/2026-09-25-bible-da.md` § 8, ADR `docs/decisions/0065-boutons-et-icones-enlumines.md`
(§ DA7c). Plafond de dépense **4 $** (distinct des 5 $ de DA5, déjà consommés à 4,84 $).

## État : TERMINÉ (2026-09-26)

1. Inventaire : les 59 traits (`data/traits/*.json`) n'avaient pas d'icône propre ; les
   4 points d'affichage (`RichTooltip.trait_tip`, `character_sheet._fill_traits`,
   `Encyclopedia._entry_icon`, `Encyclopedia._trait_fiche`) utilisaient tous
   `"trait_category_" + category` (icône de catégorie générique, 5 valeurs).
2. **Données** : 59 entrées ajoutées à `data/ui/icons_ink.json["icons"]`, `"group": "trait"`
   (ajouté à l'enum du schéma `data/schemas/icons_ink.schema.json`), `id == targets[0] ==`
   l'identifiant du trait (ex. `trait_admiral`) — `IconLibrary.get_icon("trait_admiral")` prend
   l'icône directement, sans passer par une catégorie.
3. **Outil réutilisé** (`tools/cent_ans_tools/ink_icons.py`, `cent-ans assets ink-icons`), pas
   de nouvel outil : `Entry.group`, filtre `group=` sur `entries()`/`plan()`,
   `lot_spent(ledger, prefix=)` (défaut `"DA5 :"`, rétro-compatible). La commande CLI gagne
   `--group`, `--subject`, `--budget-cap` pour que DA7c ait sa propre ligne de grand livre et
   sa propre enveloppe sans toucher au plafond DA5 déjà quasi épuisé.
4. **Génération réelle** : pas bloquée par le garde-fou de permissions (contrairement à DA2).
   ```
   uv run --project tools cent-ans assets ink-icons --group trait \
     --subject "DA7c : icônes de trait à l'encre" --budget-cap 4.0
   ```
   59 images, réel **2,6729 $** (estimé 2,6845 $), sous le plafond de 4 $. Ligne auto-ajoutée à
   `docs/budget.md` (cumul DA 10,64 $). Sources brutes dans `tools/da5_raw/icons/trait_*.jpg`,
   PNG dans `game/assets/icons/ink/trait_*.png`, `index.json` mis à jour (build automatique en
   fin de commande).
5. **GDScript** : les 4 sites passent l'id du trait plutôt que `"trait_category_" + category` ;
   repli sur l'icône de catégorie générique conservé (`IconLibrary.resolve`, inchangé) pour un
   trait sans icône propre.
6. **Tests** : `tools/tests/test_ink_icons.py` (11 tests : schéma, groupe, prefix du grand
   livre, un trait de `data/traits` = une entrée du catalogue). `game/tests/smoke.gd` (F2)
   vérifie en plus que chaque trait a sa propre icône (pas seulement celle de sa catégorie).
   `godot --headless --path game --script res://tests/smoke.gd` : exit 0, `smoke OK: icons,
   216 entries loaded, 201 data ids covered…`.
7. Capture `docs/img/da7c/fiche_traits.png` (`game/tests/da7c_traits_shot.gd`, fiche d'Édouard
   III, 3 traits avec chacun sa propre pastille).
8. `docs/decisions/0065-boutons-et-icones-enlumines.md` § DA7c ajoutée.

## Suites possibles (non faites)

- Le bandeau de cour (liste des personnages) n'affiche pas les traits eux-mêmes (seulement le
  portrait) : rien à brancher là.
- Un futur trait ajouté à `data/traits/` sans icône générée retombe silencieusement sur
  l'icône de sa catégorie (`IconLibrary.resolve`) : pas d'alerte de build si on oublie de
  l'ajouter à `icons_ink.json`. Un test `smoke.gd`/pytest le détecterait (déjà en place).
