# HUD de campagne inspiré de Total War — composants (lot F10a) et guide d'intégration

Référence : `docs/design/2026-09-23-audit-ui-total-war.md` § 3.1 et § 3.3.
Captures : `docs/img/hud-preview.png` (1440×900, décision bloquante), `docs/img/hud-preview-1920.png`
(1920×1080, lettre dépliée, deux régiments sélectionnés, sans décision), `docs/img/hud-preview-20units.png`
(20 régiments sur deux rangs, armée sans chef).

On reprend la **grammaire** de TW:WH3 (bandeau d'armée en bas au centre, médaillon du général en bas à
gauche, cercle de fin de tour et alertes en bas à droite, messages empilés à droite) dans le registre du
manuscrit enluminé : parchemin opaque, encre sépia, rubriques rouges, filets d'or, cire à sceller, écus
exacts (`game/assets/heraldry/fac_*.png`). Ni lueur, ni ornement fantastique.

## 1. Fichiers

| Fichier | Rôle |
|---|---|
| `game/scripts/ui/hud_style.gd` (`HudStyle`) | palette (reprise de `parchment_theme.tres`), boîtes de style, sceau de cire, découpe circulaire sans shader, pictogrammes de repli dessinés au trait, accès facultatif à `IconLibrary` |
| `game/scripts/ui/army_strip.gd` + `game/scenes/ui/army_strip.tscn` (`ArmyStrip`) | bandeau d'ost |
| `game/scripts/ui/general_seal.gd` + `game/scenes/ui/general_seal.tscn` (`GeneralSeal`) | sceau du chef |
| `game/scripts/ui/end_turn_cluster.gd` + `game/scenes/ui/end_turn_cluster.tscn` (`EndTurnCluster`) | cloche de fin de saison et alertes |
| `game/scripts/ui/news_letters.gd` + `game/scenes/ui/news_letters.tscn` (`NewsLetters`) | lettres scellées |
| `game/tests/hud_preview.tscn` / `.gd` | aperçu sur fond de carte, données de 1337, `-- --screenshot=<png>` |
| `game/tests/hud_components_test.gd` | test headless (instanciation, API, signaux) |

Aucun composant n'accède à `SimFacade` ni à `CampaignSim` : ils reçoivent des `Dictionary` et émettent
des signaux. Icônes : si l'autoload `IconLibrary` (F2) existe, `HudStyle.icon(id, category)` l'utilise
(`unit_<type>` puis `class_<catégorie>` pour les cartes, `hud_<alerte>` pour les pastilles,
`hud_stance_*`, `hud_supply`, `hud_movement` pour le sceau) ; sinon pictogrammes dessinés.

Commandes :

```sh
godot --path game res://tests/hud_preview.tscn -- --screenshot=docs/img/hud-preview.png
godot --path game --resolution 1920x1080 res://tests/hud_preview.tscn -- --screenshot=docs/img/hud-preview-1920.png --expand --select=5,6 --no-block
godot --path game res://tests/hud_preview.tscn -- --units=20 --no-general
godot --headless --path game --script res://tests/hud_components_test.gd
```

## 2. API

### ArmyStrip (bas centre)
- `set_army(army: Dictionary, capacity := 20, catalog := {}, title := "")` — `army` = `CampaignSim.get_army(id)`
  tel quel ; `catalog` = `ArmyStrip.load_unit_catalog(MapPaths.data_dir)` (classe, entretien, moral de base,
  lu une fois dans `data/unit_types/*.json`) ; `title` = rubrique (« Ost de Philippe VI »).
- `clear()`, `select(indices)`, `get_selection() -> PackedInt32Array`, `total_upkeep()`, `card_layout(n)`,
  propriétés `max_width` (px disponibles ; les cartes rétrécissent puis passent sur deux rangs) et `can_split`.
- Signaux : `unit_selected(index)`, `selection_changed(indices)`, `split_requested(indices)`.
- Clic = sélection simple ; Maj/Ctrl/Cmd-clic = ajoute/retire ; « Séparer (n) » actif si 0 < n < total.
- Moral en liseré (bande gauche, hauteur et teinte = moral / moral de base du type) ; effectif en
  chiffres + barre ; infobulle riche (nom, classe, effectif, moral/base, entretien).
- Noms : retour à la ligne aux espaces seulement ; police 12 → 10 ; sinon abréviation de chancellerie
  (« Arbalét. ») ; sous 70 px de large, carte compacte sans nom (nom dans l'infobulle).

### GeneralSeal (bas gauche)
- `set_general(character: Dictionary, army := {}, faction := "")` — `character` = `CampaignSim.get_character(army.general)`
  (vide = « Sans chef ») ; `army` = `get_army(id)` pour posture, ravitaillement, mouvement.
- Portrait : `character.portrait` (chemin) sinon `PortraitLoader.portrait_texture(id)`, découpé en disque ;
  à défaut, écu de la faction sur le sceau.
- Rang : `character.rank` s'il existe, sinon `1 + skills_learned.size()` (affichage seulement).
- Pastille rouge = `skill_points` non dépensés.
- Signaux : `general_requested(character_id)` (clic sur le composant), `stance_selected(stance)` (clic sur la
  posture → menu Normale / Chevauchée / Siège ; propriété `can_change_stance`, faux pour une armée étrangère).

### EndTurnCluster (bas droite)
- `set_date(label, turn := -1)` (« Printemps 1337 » : saison sur la 1ʳᵉ ligne, année sur la 2ᵉ).
- `set_alerts(alerts: Array)` avec `{kind, text, province_id?, army_id?, character_id?, blocking?}` ; types :
  `chronicle_decision` (bloquante par défaut), `enemy_army`, `siege`, `debt`, `idle_character`,
  `construction_done`, `research_done` (autres : pastille neutre). Une pastille par type (compteur si > 1),
  7 au plus, en éventail ; cire rouge = danger, parchemin = information.
- `set_end_turn_enabled(bool)`, `blocking_alert()`, `EndTurnCluster.is_blocking(alert)`, `get_alert_groups()`,
  `activate_group(i)`, `fan_left_edge()` (borne gauche de l'éventail, pour placer le bandeau).
- Signaux : `end_turn_requested` (clic ou action `campaign_end_turn`, s'il n'y a aucune alerte bloquante),
  `alert_activated(alert)` (clic sur une pastille : alertes du type à tour de rôle ; clic sur la cloche quand
  une décision bloque : l'alerte bloquante).
- Propriété `use_shortcut` (vrai) : le bouton porte le raccourci `campaign_end_turn` (Entrée).

### NewsLetters (haut droite, sous la minicarte)
- `push_news({kind, title, text, faction_id?, province_id?})` (la plus récente en haut ; 40 conservées),
  `NewsLetters.news_from_event(event)` (événement du journal → lettre, `{}` si l'événement n'en mérite pas),
  `activate(i)`, `dismiss(i)`, `clear()`, `get_items()`, propriété `max_visible` (5).
- Types : `faction_met`, `war_declared`, `peace`/`peace_signed`, `birth`, `death`, `succession`,
  `city_captured`/`province_captured`, `alliance_formed`, `alliance_broken`, `marriage`, `vassalage`,
  `vassal_rebellion`, `faction_destroyed`, `excommunication`, `regency`, `general_captured`, `revolt`, `plague`.
  Cire verte pour la paix et les alliances, brune pour un décès, rouge sinon.
- Signaux : `news_activated(item)` (clic gauche, déplie le texte), `news_dismissed(item)` (clic droit).

## 3. Guide d'intégration (à faire par l'orchestrateur)

Fichiers à modifier : `game/scripts/map/map_ui.gd` (+ sa scène dans `campaign_map.tscn`) et
`game/scripts/map/campaign_map.gd`. Les composants n'ont besoin d'aucune modification.

### 3.1 Placement dans `map_ui`
Ajouter les quatre scènes comme enfants du `CanvasLayer` du HUD, puis reproduire `layout_hud()` de
`game/tests/hud_preview.gd` sur `resized` du HUD et `minimum_size_changed` du bandeau (appel différé) :
- sceau : `(16, H − hauteur − 16)` ;
- cloche : coin bas droit, marge 8 ;
- bandeau : `max_width = bord gauche de l'éventail (cluster.position.x + cluster.fan_left_edge() − 10) −
  (bord droit du sceau + 16)`, centré dans cet intervalle, marge basse 12 ;
- lettres : x = `W − 300 − 16`, y = bas de la minicarte F6 + 10 (en attendant F6 : barre haute + 8).
Le bandeau et le sceau ne sont visibles que lorsqu'une armée est sélectionnée.

### 3.2 Ce qui remplace quoi
| Actuel | Remplacé par | Remarque |
|---|---|---|
| `ArmyPanel` (panneau de droite) : liste texte des unités | `ArmyStrip` | supprimer `%ArmyPanel` du HUD ou le garder masqué le temps de la transition |
| `ArmyPanel` : général, compétences, mouvement, ravitaillement, posture | `GeneralSeal` | posture : `stance_selected` remplace l'`OptionButton` |
| `ArmyPanel` : position, ordre en cours | étiquette de survol / chemin (`%HoverLabel`) | la remonter au-dessus du bandeau (y = haut du bandeau − 32) |
| `%EndTurnButton` (barre haute) | `EndTurnCluster` | **retirer le raccourci de l'ancien bouton** (sinon double fin de tour), puis le bouton |
| `%DateLabel` | garder dans la barre haute (date en toutes lettres) **et** `cluster.set_date(...)` | |
| `%EventLog` (journal bas gauche) | reste accessible par un bouton « Journal » de la barre haute ; se replie par défaut | il chevaucherait le sceau : le déplacer à gauche au-dessus du sceau ou en fenêtre |
| — | `NewsLetters` | nouvelles marquantes seulement ; le journal garde tout |

### 3.3 Données → composants (`campaign_map.gd`)
- **Sélection d'armée** (`select_army(army_id)`, après `ui.show_army`) :
  ```gdscript
  var army: Dictionary = sim.call("get_army", army_id)
  var general_id := str(army.get("general", ""))
  var character: Dictionary = sim.call("get_character", general_id) if general_id != "" and _characters_available() else {}
  ui.army_strip.set_army(army, ARMY_CAPACITY, _unit_catalog, "Ost de %s" % (str(army.get("general_name", "")) if general_id != "" else SimFacade.faction_short_name(faction)))
  ui.army_strip.can_split = is_player
  ui.general_seal.can_change_stance = is_player
  ui.general_seal.set_general(character, army, faction)
  ```
  `_unit_catalog = ArmyStrip.load_unit_catalog(MapPaths.data_dir)` une fois au chargement.
  `deselect_army()` → `army_strip.clear()` et masquer le sceau. Rafraîchir aussi après chaque ordre,
  fin de tour et chargement (comme `show_army` aujourd'hui).
- **Capacité** (`8/20`) : aucune limite n'existe encore dans `core/`. En attendant une règle (liée au rang
  ou au Commandement du chef, lot F4), passer 20 ; la valeur doit venir de la simulation ou de `data/`,
  pas d'une constante GDScript durable.
- **Fin de tour** : `cluster.end_turn_requested` → `_on_end_turn()` (même chemin que `ui.end_turn_pressed`) ;
  `map_ui.set_end_turn_enabled(x)` doit aussi appeler `cluster.set_end_turn_enabled(x)`.
  Après chaque tour / chargement : `cluster.set_date(sim.get_date_label(), sim.get_turn())`.
- **Alertes** (F3 les produira ; en attendant, construction minimale côté `campaign_map`, sans règle) :
  - `sim.get_pending_decisions()` → `{kind: "chronicle_decision", text: title, province_id: province, blocking: true, decision_id: id}` ;
  - événements du tour de `kind` `building_completed` → `construction_done`, `technology_researched` (faction du joueur) → `research_done`,
    `siege_started` (province du joueur) → `siege` ;
  - `get_faction_summary(player).treasury < 0` ou `projected_income < 0` → `debt` ;
  - armées étrangères en guerre avec le joueur dans une province voisine de ses provinces → `enemy_army`
    (à calculer dans `core/` par F3, pas en GDScript) ;
  - personnages de `get_faction_characters(player)` sans `army` ni `governor_of`, adultes → `idle_character` (F3).
- **Activation d'une alerte** (`cluster.alert_activated(alert)`) :
  `chronicle_decision` → `chronicle.open_window()` ; `army_id` → `select_army(army_id)` + caméra sur l'armée ;
  `province_id` → caméra (`camera_rig.look_at_point`) + panneau de province ; `character_id` →
  `_on_character_selected(id)`.
- **Nouvelles** : dans `ui.add_events(events, …)` (ou juste avant), pour chaque événement
  `var item := NewsLetters.news_from_event(event)` ; si non vide, `news_letters.push_news(item)`. Une
  rencontre de faction (`faction_met`) n'existe pas encore comme événement : à produire par F3/F4.
  `news_activated(item)` → caméra sur `item.province_id` s'il existe, sinon ouvrir la diplomatie sur
  `item.faction_id`.
- **Sceau** : `general_requested(id)` → `_on_character_selected(id)` (fiche) ; si `id` est vide, ouvrir la cour
  filtrée sur les chefs (`CourtPanel.FILTER_GENERAL`). `stance_selected(stance)` →
  `_submit({"type": "set_stance", "army": selected_army, "stance": stance}, "Posture modifiée.")` (chemin actuel de `stance_changed`).
- **Bandeau** : `split_requested(indices)` → `submit_order({"type": "split_army", "army": selected_army,
  "unit_indices": Array(indices)})` puis `refresh_all()` ; `unit_selected(i)` → rien d'obligatoire (plus tard :
  détail d'unité, F2).

### 3.4 Tests
- Ajouter l'appel du test composants à la chaîne de tests (il est autonome) ; le smoke n'a pas été modifié.
- Après intégration, recapturer `campaign_map.tscn -- --screenshot=...` à 1440×900 et 1920×1080 et vérifier
  l'absence de chevauchement avec la minicarte (F6), le journal et l'étiquette de chemin.

## 4. Limites connues
- La capacité `8/20` et le rang n'ont pas de règle dans `core/` (affichage seulement).
- L'entretien affiché est la somme des `upkeep` de base du catalogue ; si `core/` applique des modificateurs,
  exposer `upkeep` dans `get_army` (le bandeau le préfère automatiquement).
- Pas de glisser-déposer pour séparer (Maj/Ctrl-clic puis « Séparer ») ; pas de fusion d'armées depuis le bandeau.
- Pictogrammes de repli volontairement sobres ; les icônes F2 les remplacent dès que l'autoload existe.
- Les tailles sont en pixels (projet sans `stretch`) : à 1920×1080 le HUD est proportionnellement plus petit.
