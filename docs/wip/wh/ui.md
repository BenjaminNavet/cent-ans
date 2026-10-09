# WH — critique `ui` : HUD et panneaux de campagne vs TWW3

Périmètre : barre du haut, panneau du seigneur, rangée d'unités, recrutement, colonie, vues d'ensemble, notifications, raccourcis, pré/post-bataille, siège. Les défauts de finition sont dans `docs/wip/rx/ui.md` (lot « uifin », ADR 0249) : non repris ici.

## 1. Résumé
- La charpente TW est déjà là : barre supérieure à infobulles budgétaires (`map_ui.gd:358-420`), sceau du chef + bandeau d'ost avec postures (`general_seal.gd`, `army_strip.gd`), barre d'emplacements de bâtiments (`settlement_slot_bar.gd`), « Mes unités » (U) et « Colonies » (B) filtrables, cloche de fin de saison à alertes cliquables (`end_turn_cluster.gd`), pré-bataille complet (jauge, verdict, renforts, retraite, `pre_battle_dialog.gd`), choix de sort de place (`capture_controller.gd`).
- Ce qui manque le plus : (1) l'information de progression des unités (expérience/rang pourtant dans le cœur) absente des cartes ; (2) aucune vue d'ensemble de faction unifiée (provinces, armées, personnages, finances, classement) : tout est éclaté en 6 fenêtres sans lien ; (3) pas de cycle « prochaine armée/colonie inactive » ni de touches de groupe ; (4) la file de recrutement n'est qu'une ligne de texte, sans annulation ; (5) après la bataille, le sort des captifs n'est pas un choix (libérer / rançonner / exécuter) mais un texte, puis une fenêtre lointaine ; (6) la barre du haut ne montre que trésor/solde/date/recherche/ferveur.

## 2. Tableau des écarts

| # | écart vs TWW3 | état actuel | impact | coût | proposition | dép. |
|---|---|---|---|---|---|---|
| 1 | Cartes d'unités sans rang/expérience ni armure/améliorations (TWW3 : chevrons, médailles) | `Unit.experience: u8` existe (`state.rs:199`, exposé `campaign_sim.rs:777`) ; `army_strip.gd` ne le lit nulle part (grep expér/xp vide) ; `tooltip_for` `army_strip.gd:360-374` : effectif, moral, entretien seulement | 4 | S | Chevrons (0-9) dessinés dans `RegimentCard._draw` ; ligne « Rang : Aguerri (3/9) » dans `tooltip_for` | aucune |
| 2 | Pas de vue d'ensemble de faction à onglets (provinces, armées, personnages, finances, classement) | `faction_panel.gd` = un seul défilement (budget, monnaie, chevalerie, rançons, féodal) ; fenêtres séparées Cour (`court_panel`), Unités (`unit_roster_controller`), Colonies (`holdings_controller`), Objectifs, Techs. Aucun « classement » : grep classement/ranking = 0 occurrence dans `game/scripts` | 4 | L | Voir §4 A : fenêtre « Royaume » à onglets qui réutilise les contrôleurs existants | #5 |
| 3 | Pas de cycle d'armées/colonies inactives ni de groupes de contrôle | `project.godot` n'a que 31 actions `map_*`/`campaign_*` ; aucune « suivante » ; la cloche signale `idle_character` mais sans touche | 4 | S | Actions `map_next_idle`, `map_next_army`, `map_next_settlement` (Tab, Maj+Tab) | aucune |
| 4 | File de recrutement : texte seul, non annulable | `settlement_panel.gd:307-323` : ligne « Recrues attendues : A, B (2 tours) », aucun bouton ; le cœur a `recruit_queue` (`economy.rs:718`, `state.rs:463`) ; unités payées d'avance | 3 | M | File en cartes (icône unité, tours restants, croix d'annulation remboursée) ; ordre `cancel_recruit` côté cœur | cœur |
| 5 | Pas de recrutement depuis le bandeau d'ost / vue unifiée du recrutement (TWW3 : onglet Recrutement avec pool, mercenaires, file globale) | recruter = ouvrir le panneau de colonie, onglet Garnison, « Recruter » ; mercenaires = bouton séparé (`army_strip.gd:42`, `hud_controller.gd:276`) ; vivier d'hommes (`recruit_pool.rs`) vu seulement via infobulles | 3 | M | Un panneau « Recrutement » unique : liste filtrable par classe, colonne vivier/coût/tours, bascule Levée/Mercenaires | cœur lecture seule |
| 6 | Barre du haut pauvre en ressources (TWW3 : une jauge par ressource avec tooltip) | `_decorate_top_bar` `map_ui.gd:208-262` : trésor, solde, date, recherche, ferveur (croisés seulement). Prestige, ordre public moyen, vivier d'hommes, ravitaillement, religion/hérésie absents | 3 | M | Ajouter 3 pastilles : prestige, hommes disponibles (somme des viviers), mécontentement moyen ou nb de provinces en risque de révolte ; infobulle riche par pastille (même gabarit `RichTooltip.hud`) | #2 |
| 7 | Panneau de colonie sans ordre public/croissance/population (TWW3 : en-tête avec ordre, croissance, richesses) | `settlement_panel.gd:104-120` : type, province, propriétaire, aux mains de, fortification, siège, revenu. Population et mécontentement vivent seulement dans le panneau de province (`province_panel.gd:227-236`) | 3 | S | Lignes « Population », « Mécontentement (révolte dans N saisons) » dans la grille de colonie, mêmes sources que `province_panel.gd` | aucune |
| 8 | Post-bataille : pas de choix de captifs (libérer / rançonner / exécuter) | `battle_result_screen.gd:455-470` n'affiche que des lignes de texte ; la rançon se fixe plus tard via « Captifs et rançons » (`faction_panel.gd:375`, `ransom_panel.gd`). Aucune action « exécuter » (grep `execute` vide dans `core` et `game`) | 4 | M | Après le résultat, une fenêtre de décision par captif de rang (comme `capture_controller`) : Rançonner (termes par défaut), Libérer sur parole (+honneur), Faire exécuter (prestige/ordre selon `chivalry_section`) | cœur : ordre `execute_prisoner` |
| 9 | Pré-bataille : prévision d'auto-résolution seule, pas de pertes estimées ; renforts passifs | `_fill_balance` `pre_battle_dialog.gd:321-348` : % de victoire + puissances ; renforts affichés (`:392-409`) sans possibilité de les exclure ; `battle_forecast.rs` ne renvoie pas de pertes | 3 | M | Ajouter `attacker_losses_pct/defender_losses_pct` (moyenne des échantillons du prévisionniste, `forecast_samples`) et les afficher sous la jauge ; case « Appeler les renforts » par armée alliée | cœur `battle_forecast.rs` |
| 10 | Flux de siège : un seul bouton « Donner l'assaut » sous la sélection | `siege_controller.gd:55-80` : ligne d'état + engins + assaut. Pas de panneau de siège (durée, attrition, vivres du camp assiégeant, sortie de la garnison, négociation de reddition) ; l'assiégé n'a pas d'équivalent défensif | 3 | M | « Fiche de siège » : vivres des deux camps par tour, brèche, engins en jauges, bouton « Sommer de se rendre » (ordre cœur) ; côté assiégé, alerte et choix de sortie | cœur |
| 11 | Notifications : cloche seulement, pas de journal d'événements filtré type TWW3 (événements du monde, bulles cliquables) | cloche `end_turn_cluster.gd` (7 types) + `news_letters`, `journal_view` (300 l.), toasts (`map_ui.gd:500`). Alertes calculées à chaque refresh (`alerts.gd`), pas d'historique des alertes passées | 2 | M | Journal à filtres (guerre, économie, personnages, diplomatie) avec clic = caméra ; vérifier d'abord `journal_view.gd` | #2 |
| 12 | Raccourcis : liste complète mais peu de commandes d'armée | `shortcut_sheet.gd:30-52` : caméra, fenêtres, filtres, partie. Rien pour poste (Maj-clic), Séparer, Garnison, stance, centrer sur armée, « fin de mouvement » | 3 | S | Actions `army_stance_*`, `army_split`, `army_garrison`, `camera_to_selection` (Espace), ajoutées à `KeyBindings.sections()` | #3 |
| 13 | Clic droit sur une carte d'unité : pas de fiche détaillée | `army_strip.gd` : aucun gestionnaire de clic droit (grep right = 0) ; l'info est une infobulle de 5 lignes | 3 | M | Clic droit ou Alt+clic → fiche (statistiques cœur : attaque, défense, charge, armure, vitesse, sources `data/units`) en `RichTooltip` épinglable | #1 |
| 14 | Tout est modal-centré ou pile de panneaux ; panneau de colonie couvre 30 % de la carte | déjà RX ui (« panneau déplaçable ») ; cité | 2 | M | déjà RX | |

## 3. Top 10 (impact / coût)

**1. Rang et expérience sur les cartes (#1)** — impact 4, coût S.
- `core/crates/godot-bridge/src/campaign_sim.rs:777` expose déjà `experience` ; vérifier que `get_army` le fournit pour chaque unité.
- `game/scripts/ui/army_strip.gd` : dans `RegimentCard._draw`, dessiner `experience` chevrons (max 9, un groupe de 3 = une barre) en haut à droite ; `tooltip_for` ajoute « Expérience : n/9 (nom du palier) ». Libellés de palier dans `data/ui/` (pas en dur).
- Test : `game/tests/` un test qui construit un `ArmyStrip` avec 3 unités d'expérience 0/4/9 et vérifie le texte d'infobulle et le nombre de chevrons (`card.chevron_count()`).

**2. Cycle de la prochaine armée/colonie à agir (#3)** — impact 4, coût S.
- `project.godot` : `map_next_idle` (Tab), `map_prev_idle` (Maj+Tab), `map_next_settlement` (Ctrl+Tab). Ajouter dans `ShortcutSheet.CAMPAIGN_SECTIONS` et `KeyBindings.sections()`.
- `game/scripts/map/unit_roster_controller.gd` : fonction `select_next_idle(step)` qui parcourt `get_army_ids` des armées du joueur avec `movement_left > 0` et sans ordre en cours, sélectionne puis centre (réutiliser le chemin du clic sur une ligne de la liste).
- Si aucune : toast « Aucune armée inactive. » Test headless dans `game/tests/` : 3 armées, deux inactives, le cycle revient au début.

**3. Rançonner / libérer / exécuter à l'issue de la bataille (#8)** — impact 4, coût M.
- Cœur : dans `sim-campaign/src/orders/` ajouter `ExecutePrisoner { character }` avec effet prestige et ordre selon `chivalry` (voir `chivalry_section.gd` pour les valeurs existantes) ; données dans `data/`.
- `battle_result_screen.gd` : émettre le signal `captives_to_decide(list)` ; `campaign_map.gd` après le retour, ouvrir une `ChronicleWindow` (même patron que `capture_controller.gd`) avec 3 boutons par captif et effets chiffrés.
- Tests : `cargo test execute_prisoner` (état, prestige, événement) ; test GDScript sur le patron `capture_controller`.

**4. Lignes Population/Mécontentement dans le panneau de colonie (#7)** — impact 3, coût S.
- `settlement_panel.gd:104-120` : deux `_grid_row`. Lire `province_snapshot` ou `get_province_state(province)` comme `province_panel.gd:227-236`. Même infobulle `RichTooltip.gauge("unrest", …)`.
- Test : panneau rempli avec `unrest=70` affiche « 70 % » et la mention de révolte.

**5. Prévision des pertes avant bataille (#9)** — impact 3, coût M.
- `battle_forecast.rs` : durant les `forecast_samples` rejouer et cumuler les pertes moyennes par camp en % des effectifs ; ajouter `attacker_losses_pct`, `defender_losses_pct` à `BattleForecast` et au dictionnaire du pont.
- `pre_battle_dialog.gd:_fill_balance` : sous `chance_label`, « Pertes estimées : vous 18 %, ennemi 41 % » (avec la réserve existante sur la bataille jouée).
- Test Rust : à effectifs égaux, pertes ≈ symétriques ; supérieurs aux pertes d'un camp 3 fois plus fort.

**6. File de recrutement en cartes, annulable (#4)** — impact 3, coût M.
- Cœur : ordre `cancel_recruit { settlement, index }` rembourse (taux dans `data/`, ex. 100 % la première saison).
- `settlement_panel.gd:307-323` : remplacer `queue_label` par un `HFlowContainer` de petites cartes (icône via `GameCatalog`, tours restants, croix). Garder le texte « places libres ».
- Tests : `cargo test cancel_recruit` ; smoke UI pour la présence de N cartes et du signal d'annulation.

**7. Pastilles prestige / hommes / mécontentement dans la barre du haut (#6)** — impact 3, coût M.
- `map_ui.gd:_decorate_top_bar` : ajouter via le patron `_add_fervor_indicator` ; données de `get_faction_summary` (vérifier les clés `prestige`, sinon les exposer dans le pont) ; enregistrer chaque pastille dans `top_fit.register_label` (court / icône seule).
- Infobulles : clés dans `data/ui/tooltips.json` (le test de clés RX garantit leur existence).
- Test : `top_bar_fit` n'écrase pas les boutons à 1280 px (voir test existant de mise en page).

**8. Raccourcis d'armée (#12)** — impact 3, coût S.
- `project.godot` + `ShortcutSheet`/`KeyBindings` : postures, séparer, garnison, centrer. `map_ui.gd:_shortcut_input` (`:1166`) route vers `press_action`.
- Test : chaque action nouvelle apparaît dans la fiche et est rebindable (déjà testé pour les actions existantes : copier).

**9. Fiche de siège et sommation (#10)** — impact 3, coût M.
- Cœur : ordre `demand_surrender` (probabilité fonction de vivres/brèche/moral, `siege.rs:314` fournit `supplies_drain`) ; `get_assault_odds` expose déjà vivres, brèche, engins.
- `siege_controller.gd` : agrandir la boîte en panneau (jauges, tours avant reddition, bouton « Sommer »). Pas de nouveau flux de bataille.
- Test Rust : sommation refusée si vivres ≥ 50 %, acceptée à 0 % et brèche ouverte.

**10. Fiche d'unité au clic droit (#13)** — impact 3, coût M.
- `army_strip.gd` : `gui_input` du `RegimentCard`, bouton droit → `unit_details_requested(index)` ; `hud_controller.gd` ouvre un `RichTooltip` épinglé (`codex_pin_tooltip` existe) alimenté par `get_unit_type_stats(type)` (à ajouter au pont si absent, lecture de `data/units`).
- Test : carte cliquée à droite émet le signal ; la fiche contient attaque/défense/vitesse.

## 4. Refontes lourdes (L)

**A. Fenêtre « Royaume » à onglets (#2)** : Provinces (liste triable : propriétaire, revenu, mécontentement, bâtiments en cours), Armées (réutilise `UnitRosterController` en contenu d'onglet), Personnages (reprend `court_panel`), Finances (reprend `budget_table` + `treasury_chart`), Classement (nouveau : factions classées par provinces, armées, revenu, prestige, via `get_faction_summary` pour chaque faction visible ; limiter au seigneurs de rang ≥ baron pour 130+ factions jouables). Étapes : (1) coquille `KingdomWindow` + `TabContainer`, avec les 4 vues existantes déplacées sans modification ; (2) onglet Classement avec tests de tri ; (3) migrer les raccourcis U/B/C vers cette fenêtre (en gardant leur alias). Coût L mais surtout du déplacement ; risque : doublons avec le `PanelStack` (`map_ui.gd:1086`).

**B. Panneau de recrutement unifié (#5)** : une seule UI pour levée urbaine, vivier régional et mercenaires, avec la file globale du royaume et un filtre « colonie à portée » (`recruitable_in_province`, `recruit.rs:492`). Demande d'exposer le vivier agrégé par province au pont. Dépend de #4.

**C. Journal d'événements filtrable (#11)** : unifier cloche, toasts et journal en un flux typé, sauvegardé avec la partie, avec historique des alertes.
