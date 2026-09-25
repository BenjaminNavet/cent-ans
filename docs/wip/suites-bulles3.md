# Suites ouvertes de la vague 3 « bulles partout » (25/09)

Origine : fin de `docs/wip/bulles-partout.md` (« Suites ouvertes »). Le joueur a demandé de tout corriger.

| Lot | Contenu | Branche | État |
|---|---|---|---|
| SV1 Vision | fusion de `m5a-vision` dans main + portée de vision armée/ville lue depuis les données | sv1-vision | lancé |
| SV2 Coûts unités | coûts en ressources des unités réellement prélevés (cœur, pont, UI, IA) | sv2-unit-resources | lancé |
| SV3 Panneau + particules | surcoût d'import de pierre dans le panneau de province ; erreur `scale_particles` en boucle au smoke | sv3-panel-particles | fait |
| SV4 Chiffres en dur | chiffres de règles écrits en dur dans les GDScript → lus depuis le cœur / `data/` | sv4-ui-numbers | lancé |

Fusion : worktree d'intégration séparé, puis ff-only dans main (voir mémoire « shared index merges »).
Disque : 18 Go libres au lancement ; chaque agent supprime son `core/target` en fin de lot.

## SV3 — détail (fait)

- **A. Surcoût d'import** : `game/scripts/map/panel_widgets.gd` (`fill_buildable`) affiche désormais une ligne
  « Dont import : X ₶ » en rouge (`RichTooltip.RED`) sous chaque bâtiment constructible dont
  `import_cost > 0` (champ déjà exposé par `core/crates/godot-bridge/src/campaign_sim.rs`,
  `build_option_dict`). Repris par le panneau de province et l'onglet Colonies (widget partagé).
  Aucune bulle à ouvrir : le rappel est visible directement dans la liste ; la bulle
  (`RichTooltip.building`) garde le détail des ressources importées.
- **B. `scale_particles` en boucle** : cause racine identifiée — `RenderQuality._on_node_added`
  passait le `Node` de particules directement à `call_deferred`. Un effet ponctuel (ex. incendie)
  peut être libéré avant l'exécution de l'appel différé ; la file de messages de Godot refuse
  alors l'argument (« Cannot convert argument 1 from Object to Object », le pointeur libéré étant
  réutilisé entre-temps). Correction : on diffère l'ID d'instance (`get_instance_id()`) plutôt que
  l'objet, et on résout le nœud avec `instance_from_id` seulement au moment de l'appel
  (`RenderQuality._scale_particles_by_id`). Test de régression ajouté dans
  `game/tests/pf1_quality_test.gd` (`_test_particles_freed_deferred`).
- Smoke (`godot --headless --path game --script res://tests/smoke.gd`) : plus aucune erreur
  `scale_particles` ; 28 « smoke OK », exit 0.
- `godot --headless --path game --script res://tests/pf1_quality_test.gd` : OK.
- Pas de Rust modifié ; `core/build.sh` lancé une fois pour la dylib, `core/target` supprimé en fin
  de lot.

## Prochaine étape

Attendre SV1/SV2/SV4, fusionner dans `integration/suites3`, smoke + cargo test + pytest, ff dans main.
