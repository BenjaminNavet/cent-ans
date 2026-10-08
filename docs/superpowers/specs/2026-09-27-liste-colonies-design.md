# Liste des colonies (accès rapide façon Total War) — conception

Date : 2026-09-27. Demande du joueur : un accès rapide à ses colonies, aux bâtiments constructibles ou à promouvoir et aux revenus, comme la liste déroulante des colonies de Total War.

## Décisions validées

- Liste organisée par **provinces repliables**, avec les colonies en dessous.
- Un clic mène à l'élément : la caméra y va et le panneau existant s'ouvre. On ne construit pas depuis la liste.
- Informations visibles : revenus, chantiers et promotions, ordre public et menaces, garnison et recrutement.
- Filtres rapides et tri.
- Approche A : une requête de synthèse dans le cœur, et un panneau Godot séparé qui ne fait qu'afficher.

## 1. Cœur (Rust)

### Module `core/crates/sim-campaign/src/holdings.rs`

Fonction pure `holdings_overview(state: &CampaignState, data: &GameData, faction: &FactionId) -> HoldingsOverview`, sans effet de bord.

**Périmètre** : les colonies dont `controller == faction`, plus celles dont `owner == faction` et `controller != faction` (marquées `occupied`). Les provinces sont celles qui contiennent au moins une de ces colonies.

**`HoldingsOverview`**
- `treasury` : trésor de la faction.
- `net_income_last_turn` : `state.faction_net_last_turn(faction)`, ou 0.
- `settlement_income_total` : somme des `income` des colonies listées.
- `count_idle` : colonies non occupées sans chantier.
- `count_upgrade` : colonies non occupées où une construction est possible (au moins une option disponible).
- `count_endangered` : colonies en danger.
- `provinces: Vec<ProvinceRow>`.

**`ProvinceRow`**
- `province`, `name`.
- `income` : somme des revenus de ses colonies listées.
- `unrest` : `population::weighted_unrest`, arrondi.
- `revolt_seasons`, `revolt_seasons_needed`, `revolt_threshold`.
- `devastation`.
- `slots_busy` : nombre de chantiers en cours. `slots_total` : nombre de colonies non occupées.
- `settlements: Vec<SettlementRow>`, la cité d'abord, puis par identifiant (ordre de `province_settlements`).

**`SettlementRow`**
- `id`, `name`, `kind`, `is_city`, `occupied`.
- `income` : `settlement_tax(...)` arrondi. Vaut 0 en cas de siège ou d'occupation, comme dans `settlement_detail`.
- `construction: Option<{ building, name, turns_left }>`.
- `idle` : ni occupée ni en chantier.
- `options_available: Vec<{ building, name, cost, is_upgrade }>` : options de `state.buildable(data, id)` avec `available == true`. `available` comprend déjà le trésor (`build_blocker`). Vide si la colonie est occupée ou déjà en chantier.
- `is_upgrade` : le bâtiment a un `upgrades_from` qui désigne un bâtiment déjà présent dans la colonie.
- `upgrade_available` : au moins une option avec `is_upgrade`.
- `recruit_queue_len`.
- `garrison_strength`, `garrison_free` (−1 s'il n'y a pas de plafond, comme dans `settlement_detail`).
- `siege: Option<{ attacker, turns_left }>`.
- `endangered` et `danger_reasons: Vec<DangerReason>`. Les raisons sont `Siege`, `Occupied`, `RevoltCountdown` (`revolt_seasons > 0` dans la province), et `UnrestAboveThreshold` (agitation pondérée ≥ seuil de révolte). Les deux raisons de révolte valent pour toutes les colonies de la province.

### Pont : `core/crates/godot-bridge/src/campaign_sim_holdings.rs`

`#[func] fn get_holdings_overview(&self, faction: GString) -> VarDictionary` convertit `HoldingsOverview` en dictionnaire, avec les mêmes noms de champs. Il renvoie un dictionnaire vide si la faction est inconnue ou si aucune partie n'est chargée. `danger_reasons` est transmis sous forme de clés texte (`"siege"`, `"occupied"`, `"revolt_countdown"`, `"unrest"`).

### Tests Rust (`sim-campaign`)

1. Une colonie sans chantier, avec un bâtiment de base et assez d'argent pour sa promotion : `idle`, `upgrade_available` et le compteur sont justes.
2. La même colonie avec un trésor insuffisant : `options_available` est vide, `upgrade_available` est faux.
3. Une colonie en chantier : `construction` est rempli, `idle` est faux, `options_available` est vide.
4. Une colonie assiégée : `income == 0`, `endangered`, raison `Siege`.
5. Une colonie possédée mais occupée par un ennemi : elle est listée avec `occupied` et la raison `Occupied`, et exclue de `slots_total`.
6. Une province : `income` est la somme de ses colonies ; `settlement_income_total` est la somme de toutes les provinces.
7. Une faction inconnue : l'aperçu est vide.

## 2. Interface (Godot)

### `game/scripts/map/holdings_controller.gd` (`HoldingsController`)

Panneau construit en code, sur le modèle de `UnitRosterController` (sections repliables, style du manuscrit enluminé). Aucune règle de jeu : il n'affiche que ce que renvoie `get_holdings_overview`.

- **Emplacement** : en haut à gauche, environ 440 px de large, hauteur limitée avec défilement. Il occupe la même place que « Mes unités » : ouvrir l'un ferme l'autre.
- **En-tête** :
  - trésor et solde du dernier tour ;
  - filtres exclusifs `[Tout] [⚒ libre N] [▲ N] [⚠ N]`, qui correspondent aux colonies `idle`, aux colonies avec `options_available` non vide et aux colonies `endangered` ;
  - tri des provinces : Revenu (par défaut, décroissant), Nom, Ordre public (plus agitée d'abord).
- **Ligne de province** : ▸/▾, nom, revenu, `⚒ slots_busy/slots_total`, jauge d'agitation, ⚠ avec « révolte dans N saisons » si le décompte a commencé. Clic sur la ligne : on déplie ou replie. Bouton « ⌖ » : la caméra va sur la province et le panneau de province s'ouvre.
- **Ligne de colonie** : nom (la cité est marquée), revenu, chantier (« Halle · 3 t », ou « libre » en ambre si `idle`), badge ▲ (plein si `upgrade_available`, sinon contour pour une construction neuve), garnison, icône de siège ou libellé « occupée ». Clic : la caméra va sur la colonie et le `SettlementPanel` s'ouvre, par le même chemin que `SettlementLayer.settlement_selected(id)`.
- **Infobulles** : sur ▲, la liste de `options_available` (nom, coût, « promotion » si `is_upgrade`). Sur ⚠, les `danger_reasons` en français, avec le nombre de saisons pour la révolte.
- **Filtres et repli** : une province reste visible si au moins une de ses colonies correspond au filtre actif, et elle est alors dépliée. Seules les colonies qui correspondent sont affichées. Avec « Tout », les provinces sont repliées par défaut, sauf celles qui contiennent une colonie en danger. Un repli ou un dépliage manuel est gardé jusqu'à la fin de la partie (en mémoire, pas dans la sauvegarde).
- **Rafraîchissement** : à l'ouverture, au début du tour du joueur, et, si le panneau est ouvert, après une construction lancée, un recrutement ou un changement de contrôle d'une colonie. Jamais à chaque image.
- **Entrées** : action `map_toggle_holdings` sur la touche B (libre aujourd'hui ; à revérifier dans `project.godot` à l'implémentation), bouton « Colonies » dans la barre du haut à côté d'« Unités », ligne dans la fiche des raccourcis.
- **Texte** : chaînes françaises, montants avec `money.gd`.

### Tests et vérification

- `game/tests/holdings_test.gd` (headless) : le panneau s'ouvre avec l'action ; il y a autant de lignes de province que dans l'aperçu ; chaque filtre réduit la liste au bon compte ; un clic sur une colonie la sélectionne ; ouvrir le panneau ferme « Mes unités ».
- `smoke.gd` reste vert.
- `game/tests/holdings_shot.gd` écrit une capture de contrôle. Une seule lecture d'image pour juger la lisibilité.

## 3. Documentation

- `docs/manuel.md` : une section « Liste des colonies (B) ».
- `docs/archive/chantiers.md` : état et prochaine étape, mis à jour à chaque commit `wip:`.
- Pas d'ADR : cette fonction ne change pas l'architecture.

## Hors périmètre

- Construire ou promouvoir depuis la liste.
- Les colonies des autres factions.
- L'entretien par bâtiment et le détail des revenus hors impôt des colonies (déjà dans le budget de faction).
