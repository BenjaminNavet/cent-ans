# FK — Carte vivante : gens, scènes et incidents sur la vue rapprochée

Date : 2026-09-29. Chantier `FK` (« folk »). ADR réservée : **0122**. Coût cloud prévu : **0 $**.
Note de suivi : `docs/wip/fk.md`.

## 1. Constat

La vue rapprochée de la carte de campagne a déjà des saisons, des fumées, des moulins, des oiseaux,
des bateaux (CV1), des armées figurées avec bivouacs (CV2) et la météo (CM2). Il manque **des gens**
et **ce qui arrive** : personne sur les routes ni aux champs, et les événements de la simulation
(peste, révolte, disette, foire…) ne se voient pas là où ils se produisent.

Demande du joueur : « la carte est un peu triste, sans événements, sans animations sur la vue
rapprochée ». Choix faits en séance :

- les trois volets : vie ordinaire, événements existants rendus visibles, incidents à cliquer ;
- incidents **légers** (dilemmes à la Total War : on clique, on choisit, c'est réglé) ;
- scènes **discrètes** : visibles en zoomant, aucune icône ni notification pour les signaler ;
- les événements `random` rattachés à une province passent **sur la carte** au lieu de la fenêtre
  modale, avec environ 15 nouveaux incidents du quotidien ;
- rendu par les **figurines skinnées des batailles** (texture d'os, `MultiMesh`), comme CV2 ;
- **marchands en charrette sur les routes commerciales**.

## 2. Architecture

### 2.1 Cœur (`core/`, Rust)

Aucune nouvelle règle en dehors de l'expiration des décisions (§ 2.1.2).

#### 2.1.1 Scènes de province (visuel seulement)

`sim-campaign/src/map_scenes.rs` : fonction pure

```rust
pub fn map_scenes(state: &CampaignState, data: &GameData) -> Vec<MapScene>;
pub struct MapScene { province: ProvinceId, settlement: Option<SettlementId>,
                      kind: SceneKind, since_turn: u32, intensity: f32 }
```

Sources :

- **État courant** : agitation au-dessus du seuil de révolte ou révolte en cours → `revolt` ;
  dévastation → `devastation` ; siège → `siege` ; chantier en cours → `construction` ;
  recrutement en cours → `muster` ; disette (nourriture négative) → `famine`.
- **Événements récents de la chronique** : nouvelle clé optionnelle `"map_scene"` dans
  `data/events/*.json` (par exemple `evt_black_death` → `plague`, `evt_crue` → `flood`,
  `evt_disette` → `famine`, foires → `fair`, sacres, paix et mariages → `celebration`).
  Une scène issue d'un événement dure le nombre de tours fixé par
  `data/rules/map_scenes.json` (`durations` par type).

`intensity` ∈ [0, 1] : dérivée de la valeur source (agitation, dévastation, gravité de l'événement)
et décroissante avec l'âge de la scène. Déterminisme : aucune lecture du générateur aléatoire.
Même statut que la météo (ADR 0027) : purement visuelle.

#### 2.1.2 Incidents sur la carte

- Clé optionnelle `"presentation": "map" | "dialog"` dans les événements. Par défaut, un événement
  `random` à portée de province est `map`. Les événements historiques et ceux de faction restent
  `dialog`.
- La décision en attente (`chronicle.rs`, `Decision`) expose sa province et sa présentation.
- **Expiration** : une décision non tranchée à l'échéance applique l'option que l'IA choisirait
  (`ai_affordable_choice`), et non plus la première. Cela vaut pour toutes les décisions : c'est un
  changement de règle, consigné dans l'ADR 0122.
- **Contenu** : environ 15 nouveaux événements `random` à portée de province, chacun avec sa scène :
  moines quêteurs, rixe de foire, loup dans une bergerie, reliques miraculeuses, pont effondré,
  marchand détroussé, charbonniers en conflit avec le seigneur, source réputée guérisseuse, moulin
  banal contesté, bateliers en grève, jongleurs, faux-monnayeur, feu de grange, mariage de notables,
  pèlerins malades. Cible : pour le joueur, **un incident tous les 2 à 4 tours** en moyenne.
  Sources historiques citées dans `sources`, comme les événements existants.

#### 2.1.3 Pont (`godot-bridge`)

- `get_map_scenes() -> Array[Dictionary]` (`province`, `settlement`, `kind`, `since_turn`,
  `intensity`).
- Décisions en attente : ajout de `province` et `presentation` aux dictionnaires existants.
- Routes commerciales : les routes déjà exposées pour `TradeRouteLayer` suffisent (points, valeur,
  état actif ou coupé).

#### 2.1.4 Données et schémas

- `data/rules/map_scenes.json` : durées par type de scène, plafond du réservoir de figurines,
  densités (charrettes par unité de valeur commerciale, paysans par millier d'habitants), rayon
  d'activité autour du centre de la caméra.
- `data/schemas/` : `map_scene` (énumération fermée) et `presentation` ajoutés au schéma des
  événements, plus le schéma de `map_scenes.json`. Une valeur inconnue est refusée.

### 2.2 Rendu (`game/`, GDScript)

Nouveau dossier `game/scripts/map/life_folk/`, instancié par `CampaignLife` à côté de
`life_ambient.gd`, avec les mêmes crochets : `refresh(sim)` une fois par tour et
`update_view(...)` à chaque image.

| Fichier | Rôle |
|---|---|
| `folk_pool.gd` | Réservoir de figurines : un `MultiMesh` par modèle (mêmes maillages et matériau à texture d'os que `ArmyFigures`, CV2), plafond fixe (≈ 600 par défaut), actif seulement au palier proche (`ZoomTiers`), dans un rayon autour du centre de la caméra. Les accessoires (charrettes, étals…) ont leur propre `MultiMesh`. |
| `folk_routine.gd` | Vie ordinaire (§ 3.1). |
| `folk_caravans.gd` | Marchands en charrette sur les routes commerciales (§ 3.2). |
| `folk_scenes.gd` | Mise en scène des `MapScene` (§ 3.3). |
| `incident_markers.gd` | Marqueur au-dessus de chaque incident du joueur (§ 3.4). |

L'animation est entièrement en shader (phase propre à chaque figurine ; clip choisi à la pose).
Le placement est recalculé quand la caméra s'éloigne d'un seuil de sa dernière position, jamais à
chaque image. Les positions suivent `TerrainBuilder.surface_height_at`.

Options de ligne de commande : `--no-folk` (A/B), `--folk-off=routine,caravans,scenes,incidents`,
`--scene=<province>:<kind>` (forcer une scène pour les tests et les captures).

### 2.3 Assets (Blender, 0 $)

- `tools/blender_scripts/folk_props.py` → `game/assets/models/folk/*.glb` : charrette de marchand
  bâchée, charrette de morts, charrette de pierres, étal, bûcher, fourche et torche (tenues en
  main), petit troupeau (moutons, vaches), croix et bannière de procession, échafaudage.
- `tools/blender_scripts/battle_skinned.py` : 3 clips de plus, `scythe` (faucher), `carry`
  (porter), `plough` (labourer), branchés dans la table des clips.
- Figurines civiles : variantes sans armure des figurines de milice existantes (livrée terne,
  sans arme ou avec un outil).

## 3. Contenu visuel

Tout est visible **au palier proche seulement**, sauf le marqueur d'incident (§ 3.4).

### 3.1 Vie ordinaire (`folk_routine.gd`)

- **Routes** (rubans de `RoadRenderer`) : piétons, cavaliers isolés et charrettes de paysans ;
  densité proportionnelle à la population des colonies reliées, réduite par la dévastation.
- **Champs** (canal R du masque de terroirs) : travaux selon la saison (`campaign_season`) :
  labour au printemps, fauche en été, vendange en automne dans les vignes (canal G), peu de monde
  l'hiver.
- **Pâtures** (canal A) : troupeaux et un berger.
- **Forêts** en hiver : bûcherons.
- **Pèlerins** : petits groupes marchant vers les colonies de niveau cité (cathédrale).

### 3.2 Marchands sur les routes commerciales (`folk_caravans.gd`)

- Pour chaque route commerciale **active** : des charrettes de marchand bâchées avec charretier,
  marchand à pied et, au-delà d'une valeur seuil, un ou deux gardes armés. Le nombre de charrettes
  dépend de la valeur de la route.
- Tracé : on suit les rubans de routes (`RoadRenderer`) quand ils relient les deux bouts, sinon la
  polyligne de la route commerciale. Les charrettes vont dans les deux sens, avec des phases
  décalées.
- Route coupée (guerre, embargo) : aucune charrette.
- Tronçons maritimes : déjà servis par les navires de `life_ambient.gd`, qui sont réutilisés (pas
  de doublon).
- Visible même quand la couche des routes commerciales (touche V) est masquée : c'est de la vie,
  pas une information de filtre.

### 3.3 Scènes de province (`folk_scenes.gd`)

Placées au plus près de la colonie touchée, sinon au chef-lieu de la province. L'intensité règle
le nombre de figurants.

| Scène | Mise en scène |
|---|---|
| `plague` | charrette de morts et deux porteurs, flagellants en procession, fumée de bûcher ; fumées de cheminée coupées dans la colonie |
| `famine` | file de gens devant l'église ; champs voisins sans travailleurs |
| `revolt` | attroupement à fourches et torches devant la halle ou le château, fumée |
| `devastation` | fuyards avec baluchons sur les routes (les ruines existent déjà, CV1) |
| `siege` | charrettes de ravitaillement vers le camp, fourrageurs dans les champs voisins (le camp existe déjà, CV2) |
| `construction` | échafaudage, maçons qui portent, charrette de pierres |
| `fair` | étals, foule, bétail, étendards |
| `celebration` | procession avec croix et bannières ; cloches si AU1 en fournit le son |
| `flood` | nappe d'eau étalée autour du fleuve (décal), gens sur les toits |
| `muster` | recrues à l'exercice près de la ville |

Les scènes existent partout (y compris à l'étranger) mais ne sont signalées nulle part : on les
découvre en zoomant.

### 3.4 Incidents (`incident_markers.gd`)

- Chaque décision en attente du joueur avec `presentation = map` pose sa scène (§ 3.3, ou une
  scène propre à l'incident, par exemple des brigands au bord de la route) et un marqueur (sceau de
  cire avec pictogramme), visible **à tous les zooms** au-dessus de la province.
- Clic sur le marqueur ou sur la scène → la fenêtre de décision existante, ancrée sur la province.
- Le marqueur montre le nombre de tours restants avant expiration ; à l'expiration, l'option de
  l'IA s'applique (§ 2.1.2) et une notification rapporte ce qui a été décidé.
- La fenêtre modale en début de tour ne s'ouvre plus pour ces décisions.

## 4. Hors champ

- Pas de son propre à chaque scène au-delà de ce qu'AU1 fournit déjà.
- Pas de scène pour les événements sans lieu (diplomatie, dynastie).
- Pas d'icône ni de notification pour signaler les scènes (choix « discrètes »).
- Pas d'incident qui exige d'envoyer une armée ou un agent.
- Pas de cycle jour-nuit.

## 5. Performance

- Cible : au plus **5 % de perte d'images par seconde** au palier proche (distance 45), mesurée en
  A/B avec `--no-folk` via `--fps-probe`, 3 paires alternées comme CV1. Aucun coût au-delà du
  palier proche, sauf les marqueurs d'incident.
- Plafond du réservoir dans `map_scenes.json` ; si `frame_budget.gd` signale un dépassement, le
  plafond est divisé par deux.
- `get_map_scenes` et la relecture des routes : une fois par tour.

## 6. Erreurs

- Scène sans colonie résolue : repli sur le chef-lieu ; sans chef-lieu, scène ignorée avec un
  avertissement unique dans le journal.
- Route, ruban ou canal de terroir absent : la routine concernée est sautée, avec un avertissement
  unique.
- `map_scene` ou `presentation` inconnus dans un fichier de données : refusés par le schéma (test
  pytest).

## 7. Tests

- `cargo test` (`core/crates/sim-campaign/tests/fk_map_scenes.rs`) :
  - `map_scenes` est déterministe et produit le bon type pour chaque source (révolte, siège,
    chantier, dévastation, événement récent avec `map_scene`), avec intensité décroissante ;
  - une décision expirée applique l'option de l'IA ;
  - un événement `random` de province est `map` par défaut, un historique reste `dialog` ;
  - une campagne simulée de 1337 à 1400 (graine 1337) donne au joueur en moyenne entre 1 incident
    tous les 2 tours et 1 tous les 4 tours.
- pytest : schémas des événements et de `map_scenes.json`.
- Godot headless `game/tests/fk_folk_test.gd` : réservoir plafonné ; routines, charrettes et scènes
  instanciées au palier proche et vides au loin ; aucune charrette sur une route coupée ; marqueur
  d'incident présent, clic → fenêtre de décision.
- `smoke.gd` vert.
- Captures de preuve `game/tests/fk_shot.gd` (route commerciale, scène de peste, incident), 3 au plus.

## 8. Lots

| Lot | Objet | Agent | Dépend de |
|---|---|---|---|
| FK0 | Squelette (fichiers vides, tests désactivés), ADR 0122, note `docs/wip/fk.md` | orchestrateur | — |
| FK1 | Cœur : `map_scenes`, `presentation`, expiration par l'option de l'IA, pont, schémas, `map_scenes.json` | cent-ans-dev | FK0 |
| FK2 | Assets : `folk_props.py`, 3 clips, figurines civiles | cent-ans-dev | FK0 |
| FK3 | `folk_pool`, vie ordinaire, marchands sur les routes commerciales | cent-ans-dev | FK0 (maquettes provisoires en attendant FK2) |
| FK4 | Scènes de province (10 types) | cent-ans-dev | FK1, FK2, FK3 |
| FK5 | Incidents : marqueurs, clic → fenêtre, notification d'expiration (cent-ans-dev) ; 15 nouveaux événements (cent-ans-mech) | cent-ans-dev + cent-ans-mech | FK1, FK3 |
| FK6 | Mesures de performance, captures, relecture, fusion dans `main` | orchestrateur | tous |

Vague 1 : FK1, FK2, FK3 en parallèle (worktrees). Vague 2 : FK4, FK5. Puis FK6.
