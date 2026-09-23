# Cent Ans — Document de conception (v1)

Date : 2026-09-23. Statut : validé par Benjamin au kickoff.

## 1. Vision

Jeu de grande stratégie solo inspiré de Total War, situé pendant la guerre de Cent Ans
(début 1337). Deux couches : une **carte de campagne** au tour par tour (un tour = une saison)
et des **batailles en temps réel avec pause** qui se déclenchent quand deux armées se
rencontrent. Le jeu vise le réalisme historique : géographie réelle, toponymes d'époque,
personnages réels, équipement, héraldique et architecture du XIVe-XVe siècle.

## 2. Décisions de cadrage

| Sujet | Décision |
|---|---|
| Moteur | Godot 4.7 (GDScript) pour rendu/UI, Rust via GDExtension (crate `godot`) pour la simulation |
| Factions jouables | France, Angleterre, Duché de Bourgogne |
| Date de départ | Printemps 1337 |
| Carte | Europe de l'Ouest élargie : France, Îles britanniques, Pays-Bas, Rhénanie, Suisse, Italie du Nord, Ibérie |
| Temps | 1 tour = 1 saison (4 tours/an) |
| Batailles | Temps réel avec pause, ~20 unités/camp de 60-120 soldats |
| Naval | Transport maritime abstrait uniquement, pas de bataille navale |
| Multijoueur | Non (architecture déterministe conservée) |
| Visuel | 3D stylisée low-poly réaliste, UI style manuscrit enluminé / parchemin |
| Assets | 3D procédurale par scripts Blender ; images (portraits, icônes, textures, concepts) via OpenRouter |
| Budget cloud | 50 $ pour la v1, suivi dans `docs/budget.md`, pas de demande en dessous |
| Plateforme | macOS uniquement |
| Langue | Code et identifiants en anglais, UI, docs et communication en français |
| Données | JSON/YAML lisibles et moddables, validés par schéma |
| Tests | Tests unitaires Rust (`cargo test`), GUT pour GDScript, smoke tests Godot headless |
| Suivi | Git + GitHub privé, `docs/status.md`, `docs/roadmap.md`, ADR dans `docs/decisions/` |

## 3. Architecture

### 3.1 Principe : cœur headless

Toute la logique de jeu vit dans le crate Rust `core`, pur et déterministe :
`state + orders + seed -> new state + events`. Godot ne contient aucune règle de jeu ;
il affiche l'état et convertit les entrées en ordres. Bénéfices : tests rapides sans moteur,
rejouabilité exacte (sauvegarde = état initial + liste d'ordres, ou snapshot), travail
parallèle des agents sans conflits entre simulation et présentation.

### 3.2 Arborescence

```
core/                Rust workspace
  crates/
    sim-campaign/    état de campagne, économie, diplomatie, personnages, techs, auto-résolution
    sim-battle/      simulation de bataille temps réel (tick fixe), sièges
    ai/              IA de campagne et de bataille
    data-model/      types sérialisables (serde) + chargement/validation des données
    godot-bridge/    GDExtension : expose sim-campaign et sim-battle à Godot
game/                projet Godot 4.7 (GDScript) : scènes, UI, rendu, caméra, input
data/                provinces, factions, unités, bâtiments, techs, personnages (JSON/YAML)
  schemas/           JSON Schema de chaque type de données
tools/               Python (uv) : pipeline géo, générateurs Blender, génération d'images, budget
assets/              assets binaires versionnés (glTF, PNG, polices) ; assets/src = sources .blend
docs/                design, décisions (ADR), status, roadmap, budget, comptes rendus de session
```

### 3.3 Flux de données

1. `tools/geo` transforme Natural Earth + Wikidata en `data/map/provinces.json`,
   `heightmap.png`, `rivers.geojson`, `cities.json`.
2. `data-model` charge et valide `data/` au démarrage (erreurs explicites).
3. `sim-campaign` produit un `CampaignState` ; Godot le lit via `godot-bridge`
   (API en lecture par identifiants, ordres en écriture, événements en file).
4. Quand deux armées se rencontrent, `sim-campaign` émet `BattleRequested` ;
   le joueur choisit bataille 3D (`sim-battle` + scène Godot) ou auto-résolution ;
   le résultat retourne dans `sim-campaign`.

### 3.4 Interface Godot <-> Rust

- Objets exposés : `CampaignSim` (new/load/save, `submit_order`, `end_turn`, getters),
  `BattleSim` (setup, `tick(dt)`, `issue_command`, getters positions/états par unité).
- Transfert par valeurs simples et `PackedArray`s, jamais de pointeurs partagés.
- Rendu des soldats par `MultiMeshInstance3D` alimenté à chaque tick par des tableaux
  de transformations.

## 4. Systèmes de jeu

### 4.1 Carte de campagne
Provinces (100-140) avec capitale, terrain dominant, rivières, ressources, routes,
côte/port. Mouvement en points d'action modulé par terrain, saison, routes et rivières.
Ravitaillement : les armées vivent sur le pays ; attrition en hiver et en territoire ravagé.
Chevauchée : ordre de pillage qui ruine la province et rapporte du butin.
Transport maritime : traversée abstraite entre ports (coût, délai, risque).

### 4.2 Villes et économie
Population par classes : paysans, bourgeois, clergé, noblesse. Chaque classe a
mécontentement, santé, richesse, satisfaction en biens. Bâtiments par catégories :
production, commerce, militaire, religieux, sanitaire (hôtel-Dieu, adduction), fortifications.
Ressources naturelles par province (blé, laine, vin, sel, fer, bois, pierre).
Le recrutement puise dans la classe correspondante (archers chez les paysans libres,
hommes d'armes et chevaliers chez la noblesse, milices chez les bourgeois).

### 4.3 Personnages et dynasties
Personnages réels de 1337 avec dates réelles ; naissances, mariages, morts simulés.
Arbre de compétences à trois branches : Commandement, Gouvernance, Cour. Traits acquis
(blessures, réputation, piété). Succession selon la loi de chaque royaume.

### 4.4 Diplomatie
Guerre, paix, alliances, trêves datées, mariages et prétentions (casus belli),
commerce et embargos, vassalité avec loyauté et rébellion. Attitude des IA calculée
à partir d'intérêts, menace, parenté, religion et historique.

### 4.5 Religion
Papauté (Avignon puis Grand Schisme en 1378 : obédience Rome/Avignon), piété des
personnages, bâtiments religieux (ordre public, santé), hérésies (Lollards, Hussites),
ordres militaires.

### 4.6 Technologie
Deux arbres, militaire et civil, financés par points issus des universités, guildes,
monastères. Militaire : arc long, plates, bombardes, artillerie de campagne, etc.
Civil : moulins, comptabilité, fortifications, hygiène, imprimerie tardive.

### 4.7 Batailles
Unités : infanterie (hommes d'armes, piquiers, milices), tir (archers, arbalétriers,
archers montés), cavalerie (chevaliers, sergents), engins (trébuchet, mangonneau,
bombarde, tour de siège, bélier). Simulation par unité avec moral, fatigue, formation,
flancs, terrain (pente, boue, forêt, gués), météo (pluie → arcs), munitions.
Sièges : murailles, brèches, échelles, sape, famine, sortie.

### 4.8 IA
Campagne : objectifs stratégiques par faction, évaluation de menace, diplomatie
utilitaire. Bataille : plans par rôle d'unité, réactions aux flancs et à la déroute.

## 5. Jalons

| Jalon | Livrable jouable |
|---|---|
| M0 Fondations | Outils, dépôt, squelette Rust+Godot communiquant, schémas, docs |
| M1 Carte | Provinces réelles, terrain 3D, caméra, villes placées |
| M2 Boucle de campagne | Tours, armées, recrutement simple, auto-résolution, sauvegarde, UI parchemin |
| M3 Villes et économie | Classes, bâtiments, jauges, ressources |
| M4 Personnages | Dynasties, compétences, traits |
| M5 Diplomatie et religion | Toutes les mécaniques diplomatiques, Papauté, Schisme |
| M6 Technologies | Deux arbres |
| M7 Batailles | Temps réel avec pause en 3D |
| M8 Sièges | Batailles de siège |
| M9 IA | IA de campagne et de bataille |
| M10 Assets et finition | Générateurs Blender, portraits, sons, événements, équilibrage |

## 6. Tests et qualité
- `cargo test` sur chaque crate, tests de propriétés (déterminisme : même seed + mêmes ordres = même état).
- GUT pour les scripts Godot critiques.
- Smoke test : lancement headless de Godot avec un script qui charge la campagne et joue 10 tours.
- ruff sur `tools/`, clippy sur `core/`.

## 7. Fonctionnement du projet
- Orchestration par agents (jusqu'à 5 en parallèle), questions posées seulement en fin de jalon.
- Pause automatique à 95 % du quota, reprise à l'heure de renouvellement.
- Chaque session : commit + push, mise à jour de `docs/status.md`, compte rendu dans `docs/sessions/`.
