# S2 — Incendies de siège : spécification

Date : 2026-09-24. Lot S2 de la vague « sièges ». **État : fait** (cœur, pont, rendu, tests). Nouvelle règle de la bataille de siège 3D
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
  (`intensity ≤ fuel / decline_fuel`). Le combustible baisse de `dt / burn_duration_s` : une maison
  brûle `burn_duration_s` × facteur météo `duration` secondes, de l'allumage à la ruine. À `fuel` = 0 :
  `burnt`, intensité 0.
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
  le feu à une maison. » ou « Les torches des … ne prennent pas : le bois est trop mouillé. ». Pour le **défenseur**,
  les maisons des faubourgs s'incendient sans condition de distance (des hommes de la garnison
  sortent par les poternes avec des torches).
- **Faubourgs** : `suburbs.count` maisons (`House.suburb = true`, rayon `suburbs.radius_m`) posées à
  `suburbs.distance_m` à l'extérieur des courtines latérales (est et ouest), réparties entre les deux,
  espacées de `suburbs.spacing_m` le long du mur. Pas devant la façade : placées là, elles barraient
  la route des tours de siège et de l'infanterie (sonde : 0 victoire sur 20 à l'échelade). Une garnison pilotée par l'IA les incendie au premier pas avec la
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
| dépassement du point d'impact / portée d'allumage / porte | 18 m / 30 m / 12 m |
| intensité initiale / croissance | 0,3 / 0,02 par s |
| durée d'une maison / déclin sous | 180 s / 30 % de combustible |
| propagation | toutes les 2 s, 0,06 à pleine intensité, bord à bord ≤ 30 m (les pâtés sont à 17-25 m), vent ≤ 0,7 |
| porte | 4 PV/s à pleine intensité pendant 200 s (≈ 600 PV : une porte de fortification 0-1 cède, 2+ tient) |
| chaleur | ≤ 8 m du bord : 0,15 % des soldats et 0,5 de moral par s à pleine intensité |
| fumée | intensité ≥ 0,3, marge 6 m, tirs ×0,5 |
| torche | à ≤ 8 m, chance 0,9, intensité initiale 0,5 |
| faubourgs | 4 maisons (rayon 8 m) à 40 m des courtines est et ouest, espacées de 34 m, incendiées par l'IA avec 50 % de chance chacune |

### Mesures (sonde, 20 graines, fortification 2)
`cargo run --release -p sim-battle --example probe -- siege <brèche> [engins]`, avec les nouveaux
interrupteurs `WEATHER=clear|rain|fog|snow` et `FIRE=off` :

| scénario | victoires assaillant | durée moyenne | maisons touchées (sur ≈ 25) |
|---|---|---|---|
| `siege 0` (2 trébuchets + tour), temps clair, feu | 19/20 | 319 s | 8,2 |
| `siege 0`, temps clair, sans feu | 20/20 | 294 s | 0 |
| `siege 0`, pluie, feu | 20/20 | 300 s | 1,4 |
| `siege 0 ""` (échelade), temps clair, feu | 9/20 | 255 s | 3,6 |
| `siege 0 ""`, temps clair, sans feu | 8/20 | 256 s | 0 |
| `siege 60`, météo tirée, feu | 19/20 | 211 s | 3,8 |

Le feu ne renverse pas l'équilibre des sièges (F5d : échelade ≈ 8/20) ; par temps sec il rallonge
l'assaut d'une demi-minute (rues brûlantes derrière la brèche), sous la pluie il reste marginal. Un
premier réglage (chaleur 0,2 %/s et 0,8 de moral/s) donnait 16/20 et 331 s : trop pénalisant pour
l'assaillant, dont l'IA ne contourne pas les rues en feu. Test du cœur : une maison allumée en
atteint en moyenne 3,2 en 300 s par temps clair, 1,2 sous la pluie.

## 4. Pont et interface
- `BattleSim.get_siege()` : chaque maison gagne `fire = {state: "intact"|"burning"|"burnt",
  intensity}` et `suburb` ; la racine gagne `gate_fire` (même forme), `wind: Vector2` (direction ×
  force), `houses_burning`, `houses_burnt`.
- `issue_command({type: "burn", units: [ids], house: i})` ou `{type: "burn", units: [ids],
  gate: true}`. Refus : maison inconnue, déjà en feu, cible absente, régiment trop loin.
- Débogage : `BattleSim.debug_ignite(house)` (la porte si `house` < 0).
- Rendu : `game/scripts/battle/siege_fire_fx.gd` (flammes et fumée `GPUParticles3D` par maison en
  feu, fumée poussée par le vent, `OmniLight3D` vacillantes plafonnées aux 8 foyers les plus intenses,
  maison brûlée effondrée — instances abaissées dans les `MultiMesh` des maisons — avec un tas
  noirci, des poutres calcinées et des braises), paramètres `data/fx/siege_fire.json` (schéma
  `data/schemas/fx_siege_fire.schema.json`, valeurs par défaut dans le script si le fichier manque).
  `battle_siege.gd` ne fait que l'appeler. Ligne HUD : « · N maison(s) en feu » à côté des brèches,
  « porte en feu ».
- Tests : `sim-battle/tests/fire.rs` (10 : règles embarquées et faubourgs, déterminisme,
  propagation plus lente sous la pluie, maison qui se consume en ruine, ruine franchissable,
  pertes et moral près d'un foyer, porte qui brûle puis cède, fumée, commande `burn`, faubourgs
  incendiés par l'IA) ; `tools/tests/test_siege_fire_schema.py` ; scène
  `game/tests/s2_fire_fx_test.gd` (flammes, lumières plafonnées, ligne HUD, ruines).

## 5. Choix consignés
- **Règles embarquées** : `data/rules/siege_fire.json` est lu par `include_str!` dans `sim-battle`
  (la bataille ne reçoit pas `GameData`, et ajouter un champ à `BattleSetup`/`SiegeSetup` aurait
  touché une quinzaine de littéraux partagés avec d'autres lots). Modifier le fichier impose de
  recompiler le cœur ; le schéma est vérifié par `tools/tests/test_siege_fire_schema.py` et la
  lecture par un test Rust. ADR `0008-incendies-regle-coeur.md`.
- **Flux RNG séparé** plutôt que le flux principal : le feu ne décale aucun autre tirage (météo,
  terrain, combats) ; à feu éteint (`set_fire_rules(None)`) une bataille est identique à avant S2,
  hors faubourgs.
- **Faubourgs sur les flancs** plutôt que devant la porte (voir § 2.2).
- **Commande `burn` du défenseur sur ses faubourgs sans condition de distance** : ses régiments sont
  derrière les murs ; on suppose des porteurs de torches sortis par les poternes.
- **Vent** : n'existait pas dans la simulation ; tiré au début de chaque siège dans le flux du feu, il
  ne sert qu'au feu (propagation et dérive de la fumée à l'écran).
- Limites connues : pas de lutte contre le feu (chaînes de seaux) ; l'IA tactique n'évite pas les
  rues en feu et n'utilise pas la commande `burn` (sauf les faubourgs au premier pas) ; le feu ne
  touche ni les engins ni les tours de siège (couverts de peaux mouillées, M8) ; l'église (modèle
  unique) ne s'effondre pas à l'écran quand son disque brûle ; pas encore de bouton « incendier »
  dans l'interface (commande disponible par le pont).
