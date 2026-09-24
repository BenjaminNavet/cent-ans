# S2 — Incendies de siège : spécification

Date : 2026-09-24. Lot S2 de la vague « sièges ». Nouvelle règle de la bataille de siège 3D
(`m8-sieges.md` § 2 et F5) : les maisons de la ville peuvent brûler. **Règle du cœur** (Rust,
`sim-battle`) ; Godot ne fait que l'afficher (ADR `0008-incendies-regle-coeur.md`).

## 1. Cadre historique
- Pendant la guerre de Cent Ans, les assiégeants lancent des **traits incendiaires** (flèches à
  étoupe) et des **pots à feu** (poix, soufre, graisse) par-dessus les murailles : trébuchets et
  mangonneaux battent la ville autant que ses murs (Calais 1346-1347, Orléans 1428-1429).
- Les villes sont faites de bois et de torchis : le feu court de maison en maison, poussé par le
  vent ; seules la pluie et la neige l'arrêtent vraiment.
- Les défenseurs **brûlent eux-mêmes leurs faubourgs** pour dégager le glacis et priver l'ennemi
  d'abris (Orléans, octobre 1428 : le faubourg du Portereau et ses églises).
- Les assaillants **brûlent la porte** (fagots entassés contre les vantaux) quand le bélier tarde.
- Murailles et tours sont en pierre : elles ne brûlent pas. La porte, en bois ferré, si.

## 2. Règles retenues (`core/crates/sim-battle/src/fire.rs`, `src/sim/fire.rs`)
Tous les nombres sont dans `data/rules/siege_fire.json` (schéma
`data/schemas/siege_fire_rules.schema.json`), embarqué dans `sim-battle` à la compilation
(`FireRules::bundled()`, voir § 5) ; une bataille peut en recevoir d'autres par
`BattleSim::set_fire_rules` (tests, sonde). Aucune règle de feu hors d'une bataille de siège.

### 2.1 État d'une maison
`SiegeWorks.houses[i].fire = { state, intensity, fuel }` :
- `intact` → `burning` → `burnt` (ruine), jamais de retour.
- À l'allumage : `intensity` = `house.initial_intensity`, `fuel` = 1.
- Chaque pas : `intensity` croît de `house.growth_per_s` × facteur météo `growth` jusqu'à 1 tant que
  le combustible dépasse `house.decline_fuel` ; ensuite elle décroît avec lui
  (`intensity ≤ fuel / decline_fuel`). Le combustible baisse de `intensity × dt / burn_duration_s`,
  la durée étant multipliée par le facteur météo `duration`. À `fuel` = 0 : `burnt`, intensité 0.
- La porte (`SiegeWorks.gate_fire`) suit la même loi avec ses propres durées (`gate`) et perd
  `gate.damage_per_s × intensity` PV par seconde ; à 0 PV, elle est ouverte comme une porte enfoncée
  (« La porte, dévorée par les flammes, s'effondre ! »). Les pans de mur et les tours ne brûlent pas.

### 2.2 Allumage
- **Tirs** : chaque volée d'un régiment de l'assaillant (tir sur une unité ou sur un pan) a une
  chance d'allumer une maison. Point d'impact = cible (unité ou milieu du pan) prolongé de
  `ignition.overshoot_m` dans l'axe du tir (les traits qui passent par-dessus le rempart). Candidats :
  maisons intactes dont le bord est à moins de `ignition.reach_m` du point d'impact, et la porte
  (intacte, pas en feu) si le point d'impact est à moins de `ignition.gate_reach_m` d'elle. On prend
  le candidat le plus proche ; chance = `ignition.by_unit_type[type]` (sinon
  `ignition.by_category[catégorie]`, sinon 0) × facteur météo `ignition`. Réglage par type : les
  engins (trébuchet, mangonneau, bombarde) lancent des pots à feu, les archers des flèches à étoupe,
  les arbalétriers moins (carreaux). Aucune unité nouvelle.
- **Commande « incendier »** (`Command::Burn { units, house | gate }`) : un régiment (ni engin ni
  machine) à moins de `torch.reach_m` du bord de la maison (ou de la porte) y met le feu, chance
  `torch.chance` × météo `ignition`, intensité initiale `torch.initial_intensity`. Journal : « Les … mettent
  le feu à une maison. » ou « Les torches ne prennent pas sous la pluie. ». Pour le **défenseur**,
  les maisons des faubourgs s'incendient sans condition de distance (des hommes de la garnison
  sortent par les poternes avec des torches).
- **Faubourgs** : `suburbs.count` maisons (`House.suburb = true`) posées à `suburbs.distance_m` en
  avant de la façade attaquée, espacées de `suburbs.spacing_m`, en laissant libre la route de la porte
  (`suburbs.gate_clearance_m`). Une garnison pilotée par l'IA les incendie au premier pas avec la
  chance `suburbs.ai_burn_chance` (chaque maison tirée séparément) : « La garnison met le feu aux
  faubourgs pour dégager les abords des murailles. ». `count` = 0 les supprime.

### 2.3 Propagation
Toutes les `spread.period_s` secondes, pour chaque maison en feu `b` (ordre des indices) et chaque
maison intacte `n` (puis la porte) dont le bord est à moins de `spread.edge_distance_m` du bord de
`b` : chance = `spread.chance_per_period` × `intensity(b)` × météo `spread` × (1 − écart /
`edge_distance_m`) × vent, vent = max(0, 1 + force × cos(angle entre le vent et la direction b → n)).
Le vent (direction uniforme, force uniforme dans [0, `spread.wind_strength_max`]) est tiré au début
de la bataille ; `wind_strength_max` = 0 donne un feu isotrope.

### 2.4 Météo (`weather`, par clé `clear` / `rain` / `fog` / `snow`)
| météo | allumage | propagation | croissance | durée |
|---|---|---|---|---|
| clair | 1 | 1 | 1 | 1 |
| brouillard | 0,8 | 0,8 | 0,9 | 1 |
| pluie | 0,2 | 0,15 | 0,5 | 0,6 |
| neige | 0,25 | 0,2 | 0,6 | 0,7 |

### 2.5 Effets
- **Chaleur** : un régiment présent dont le rectangle est à moins de `heat.radius_m` du bord d'une
  maison en feu (ou de la porte en feu) perd `heat.loss_per_s × intensité × dt` de ses soldats et
  `heat.morale_per_s × intensité × dt` de moral (la maison la plus intense compte seule). Les pertes
  entrent dans `tick_losses` (moral habituel).
- **Fumée** : un tir dont la ligne passe à moins de `rayon + smoke.margin_m` d'une maison en feu
  d'intensité ≥ `smoke.min_intensity` voit ses pertes multipliées par `smoke.accuracy_factor` (une
  seule fois). Les engins (tir en cloche) n'y sont pas sensibles.
- **Ruine** : une maison `burnt` n'est plus un obstacle — ni pour l'arrêt des régiments, ni pour la
  grille A* (`sim/pathing.rs`), ni pour le lissage ; le cache des chemins est invalidé quand le
  nombre de ruines change.

### 2.6 Déterminisme
Le feu tire dans un **flux RNG dédié** (`BattleRng::from_seed(seed ^ sel)`), créé par
`BattleSim::new` : même graine, mêmes commandes → même incendie, et le feu ne décale pas les
tirages des autres règles (pas d'effet de bord sur la météo, le terrain ou les combats).

## 3. Chiffres (`data/rules/siege_fire.json`)
| réglage | valeur |
|---|---|
| allumage trébuchet / mangonneau / bombarde | 0,18 / 0,22 / 0,10 par volée |
| allumage archers longs / archers montés / arbalétriers | 0,015 / 0,010 / 0,006 par volée |
| repli par catégorie `ranged` / `siege` | 0,01 / 0,15 |
| dépassement du point d'impact / portée d'allumage / porte | 18 m / 20 m / 12 m |
| intensité initiale / croissance | 0,3 / 0,02 par s |
| durée d'une maison / déclin sous | 180 s / 30 % de combustible |
| propagation | toutes les 2 s, 0,05 à pleine intensité, bord à bord ≤ 20 m, vent ≤ 0,7 |
| porte | dégâts 4 PV/s à pleine intensité, durée 200 s |
| chaleur | ≤ 8 m du bord : 0,2 % des soldats et 0,8 de moral par s à pleine intensité |
| fumée | intensité ≥ 0,3, marge 6 m, tirs ×0,5 |
| torche | à ≤ 8 m, chance 0,9, intensité initiale 0,5 |
| faubourgs | 4 maisons à 55 m de la façade, espacées de 34 m, route de la porte libre sur 25 m, incendiés par l'IA avec 50 % de chance chacun |

## 4. Pont et interface
- `BattleSim.get_siege()` : chaque maison gagne `fire = {state: "intact"|"burning"|"burnt",
  intensity}` et `suburb` ; la racine gagne `gate_fire` (même forme), `wind: Vector2` (direction ×
  force), `houses_burning`, `houses_burnt`.
- `issue_command({type: "burn", units: [ids], house: i})` ou `{type: "burn", units: [ids],
  gate: true}`.
- Godot : `game/scripts/battle/siege_fire_fx.gd` (flammes et fumée `GPUParticles3D` par maison en
  feu, fumée poussée par le vent, `OmniLight3D` vacillantes plafonnées aux 8 foyers les plus intenses,
  maison brûlée abaissée et noircie avec des poutres calcinées), paramètres
  `data/fx/siege_fire.json` (schéma `data/schemas/fx_siege_fire.schema.json`). `battle_siege.gd`
  ne fait que l'appeler. Ligne HUD : « · N maison(s) en feu » à côté des brèches.

## 5. Choix consignés
- **Règles embarquées** : `data/rules/siege_fire.json` est lu par `include_str!` dans `sim-battle`
  (la bataille ne reçoit pas `GameData`, et ajouter un champ à `BattleSetup`/`SiegeSetup` aurait
  touché une quinzaine de littéraux partagés avec d'autres lots). Modifier le fichier impose de
  recompiler le cœur ; le schéma est vérifié par `tools/tests/test_siege_fire_schema.py` et la
  lecture par un test Rust.
- **Flux RNG séparé** plutôt que le flux principal : les batailles de siège existantes gardent leurs
  tirages (tests d'équilibrage F5d stables à feu éteint).
- Pas de lutte contre le feu (chaînes de seaux) ni d'IA qui évite les maisons en feu : limites
  connues, voir le rapport du lot.
