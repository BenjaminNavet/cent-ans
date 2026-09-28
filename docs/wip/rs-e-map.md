# RS-E — Carte de campagne (Godot seulement) — fichier de reprise

Branche `feat/rs-e-map`, depuis `main`. Lot du chantier RS (`docs/wip/restes.md`).
Scope : Godot seulement (`game/`, `data/ui/map_legend.json` + son schéma). Aucun changement `core/`.

## État (28/09)

1. **`diplomacy_panel.gd`** : `_render_map()` lit désormais `ProvinceSnapshot.of(sim, map_data)` une
   fois, `_controller_of` prend l'instantané + un index au lieu d'appeler `get_province_state` par
   province dans la boucle (lignes ~663, ~677). `_on_map_clicked` (un seul appel, sur un clic isolé,
   pas une boucle) inchangé. Fait.
2. **`alerts.gd`** (ligne ~35) : **non modifié**. La lecture par province restante ne sert qu'aux
   places assiégées possédées par le joueur (déjà filtrées par `ProvinceSnapshot.besieged`, PB3d) et
   lit le détail du siège (`attacker`, `supplies`, `turns_elapsed`) — absent de
   `get_provinces_snapshot` (`core/crates/godot-bridge/src/campaign_sim_provinces.rs`, qui ne rend
   qu'un booléen `besieged`). Impossible à éliminer sans ajouter un champ groupé côté pont (Rust,
   hors scope de ce lot : « ne compile pas le core »). Précédent : revue-code.md n°9 avait déjà
   écarté ce point pour la même raison. Signalé comme point ouvert, à reprendre par un lot qui touche
   `core/godot-bridge`.
3. **Légende (FR1/UX1)** : nouvelle section « Frontières et territoires occupés » dans
   `data/ui/map_legend.json` (toujours affichée, comme `fog`), un nouveau type d'échantillon
   `occupation` (`legend_sample.gd`, hachures aux couleurs de l'occupant sur fond du propriétaire,
   même principe que `faction_borders.gdshaderinc::fr1_hatch_at`). Texte précise que la frontière
   n'est dessinée que sur la terre. **Frontière en mer : absente du jeu** (aucun rendu de frontière
   maritime trouvé dans `faction_borders.gdshaderinc` ni ailleurs ; les provinces ne couvrent que la
   terre), donc rien à ajouter sur ce point — le texte le dit explicitement plutôt que de laisser
   croire à un oubli. Schéma (`map_legend.schema.json`) étendu : type `occupation`, propriétés
   `owner_color`/`occupier_color`. Assertion ajoutée dans `ux1_test.gd` (`labels.has("Territoire
   occupé")`).
4. **`next_hint_controller.gd`** : `REFRESH_SECONDS` renommé `FALLBACK_SECONDS` (5 s, minuterie de
   secours). `campaign_map.gd` appelle `next_hint.refresh()` explicitement : à la fin de
   `refresh_all()` (couvre fin de tour et tout ordre, `refresh_all` étant le point de passage
   commun), dans `select_army()` / `deselect_army()` et dans `_on_province_selected()` (sélection —
   la couverture du conseil dépend des panneaux ouverts). `_process` ne fait plus qu'un filet de
   sécurité lent.

## Vérifications

- `uv run --project tools pytest tools/tests/test_map_legend_schema.py` : 3 passed.
- Import Godot fait (`godot --headless --path game --import`, dylib copiée depuis le dépôt
  principal).
- Reste à lancer avant la fin : `smoke.gd`, `ux1_test.gd`, `ux2_test.gd`, un test diplomatie/alertes
  s'il en existe un dédié à ces scripts (chercher `diplomacy`, `alerts` dans `game/tests`).

## Prochaine étape

Lancer les tests headless listés ci-dessus, corriger si besoin, puis `git merge main`, réimport,
retest, et rendre la main (rapport ≤ 10 lignes à l'orchestrateur).
