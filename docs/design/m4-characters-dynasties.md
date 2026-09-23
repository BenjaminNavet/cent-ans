# M4 — Personnages et dynasties : spécification

Date : 2026-09-23. Objectif : les personnages deviennent des acteurs. Ils gagnent de l'expérience, montent
un arbre de compétences à trois branches, acquièrent des traits, se marient, ont des enfants, vieillissent,
meurent, héritent. Les généraux et gouverneurs modifient batailles et provinces. Les traits existent déjà
dans `data/characters` (`trait_*`, vocabulaire libre) : M4 les définit dans `data/traits/`.

## 1. Données (`data/`)
- `data/schemas/trait.schema.json` et `data/traits/*.json` (≈ 30) : `id`, `name` (fr), `category`
  (personality | martial | governance | physical | acquired), `effects` (liste d'`Effect` réutilisant `EffectKind` :
  ArmyMorale, ArmyExperience, TaxIncome, Unrest, Health, Piety, Prestige, Loyalty, Supply, Movement…),
  `opposites` (ids), `description`. Couvrir tous les `trait_*` référencés dans `data/characters` (agent : lister
  avec grep) : ambitious, chivalrous, energetic, brave, wrathful, ruthless, lustful, resolute, pious, cruel, just,
  greedy, generous, cautious, reckless, scholar, diplomat, sickly, strong, wounded, maimed, one_eyed, mad (Charles VI),
  captive_ransomed, veteran, siege_master, cavalry_commander, archer_captain, admiral…
- `data/schemas/skill.schema.json` et `data/skills/*.json` (≈ 30) : arbre à trois branches
  (`command`, `governance`, `court`), `tier` 1-3, `prerequisites` (ids), `cost` (points, 1 par tier),
  `effects` (mêmes `EffectKind`, plus `BattleCharge`, `BattleRanged`, `BattleDefense`, `SiegeSpeed`,
  `RecruitCost`, `Construction Speed` → ajouter à `EffectKind` si absents : `SiegeSpeed`, `ConstructionSpeed`,
  `Diplomacy`, `Intrigue`, `Fertility`). Exemples : Commandement : Hardiesse (charge +10 %), Ordre de bataille
  (moral +5), Maître des sièges (durée -1), Chevauchée (mouvement +1) ; Gouvernance : Bon justicier (mécontentement -5),
  Intendant (impôts +5 %), Bâtisseur (construction -1 tour) ; Cour : Beau parleur (diplomatie +2, M5), Entremetteur
  (mariages), Piété (piété +10).
- Enrichir `data/characters` : vérifier que chaque faction a `ruler` et `heir` (ou héritier calculable) et que les
  familles sont reliées (père/mère/enfants) pour les maisons royales de France, Angleterre, Bourgogne, Écosse,
  Navarre, Castille, Bretagne. Ajouter les personnages manquants nécessaires à la succession jusqu'en 1360 environ
  (Jean II, Charles V enfant né 1338 → généré, Édouard de Woodstock déjà, Lionel d'Anvers 1338, Jean de Gand 1340 →
  générés par la simulation OU listés comme `historical: true` avec `birth` future : la simulation les fait naître
  à leur date réelle si les deux parents sont vivants et mariés, sinon les génère aléatoirement). Champ `expected_birth`
  n'existe pas : utiliser `birth` avec date > 1337 et `status: "unborn"` (ajouter la valeur à `CharacterStatus`).

## 2. Simulation (`core/crates/sim-campaign`, `characters.rs` + nouveau `dynasty.rs`, `skills.rs`)
- `CharacterState` gagne : `experience: u32`, `skill_points: u32`, `skills_learned: BTreeSet<SkillId>`,
  `traits: BTreeSet<TraitId>`, `spouse: Option<CharacterId>`, `children: Vec<CharacterId>`, `father`, `mother`,
  `piety: u8`, `prestige: i32`, `loyalty: u8` (vassaux, M5), `title: Option<String>` (titre principal affiché),
  `age` via `birth_year` (existant), `governor_of: Option<ProvinceId>`.
- Expérience : +10 par bataille (+20 si victoire) pour le général, +2 par tour de gouvernance ; un point de
  compétence par 100 XP × tier courant. Ordre `learn_skill { character, skill }` (prérequis, points, branche).
- Traits : acquis par événements (blessure après une défaite 15 %, `trait_veteran` après 5 batailles,
  `trait_siege_master` après 3 sièges gagnés, `trait_cruel` après 3 chevauchées, `trait_captive_ransomed` après
  captivité) ; opposés s'excluent. Effets des traits et compétences agrégés dans `character_effects(id) -> EffectTotals`
  (réutiliser `EffectTotals` de M3, l'étendre avec les nouveaux kinds).
- Application : `battle_auto` prend en compte le général (`ArmyMorale`, `BattleCharge`, `BattleRanged`,
  `BattleDefense`, plus commandement × 3 % existant) ; `province_effects` ajoute les effets du gouverneur
  (`governor_of`) ; `siege` applique `SiegeSpeed` ; mouvement applique `Movement`.
- Gouverneurs : ordre `assign_governor { province, character }` (personnage de la faction, vivant, non captif,
  non général) ; un gouverneur par province, un personnage gouverne une province ou commande une armée.
- Mariages : ordre `propose_marriage { character, spouse }` entre personnages de la même faction (mariages
  interfactions = diplomatie M5, mais le moteur doit déjà accepter deux factions) : conditions (vivants, sexes
  opposés, âge ≥ 14, non mariés, pas parent/enfant/frère). Effet : prestige +, `spouse` lié.
- Naissances : chaque hiver, couple marié avec femme 16-45 ans : probabilité 25 % × (1 + Fertility) ;
  personnage généré (nom depuis une liste par culture dans `data/names/<culture>.json` : ≈ 30 prénoms masculins,
  30 féminins par culture fr/en/nl/oc/es/it/de), compétences 0-3 aléatoires, 1 trait de personnalité. Naissances
  historiques : personnages `status: unborn` naissent à leur `birth` réelle si les parents sont vivants ; sinon
  jamais (l'histoire diverge).
- Majorité à 15 ans ; un enfant n'est ni général ni gouverneur ; régence si le dirigeant est mineur (événement,
  malus mécontentement +5 pendant la régence).
- Mort : table existante + traits (`sickly` ×2, `strong` ×0,7) + captivité ; mort au combat 5 % pour un général vaincu.
- Succession : loi de la faction (`succession_law` dans `data/factions` : `salic`, `male_preference`,
  `agnatic_seniority`, `elective`) — implémenter salic (fils aînés, puis frères, puis lignée mâle), male_preference
  (filles après fils), agnatic_seniority (frère aîné avant fils), elective (meilleur `court` de la maison) ;
  sinon plus vieux mâle de la maison (existant) ; sinon `no_heir` → la faction passe en régence perpétuelle
  (IA) ou `faction_destroyed` si aucun personnage. Événements `succession` détaillés en français.
- Prestige : + victoires, + mariages, + titres ; sert plus tard (M5 diplomatie).
- `state_version` → 3. Tests (≥ 14) : XP et points, learn_skill prérequis, trait acquis après 5 batailles,
  général avec compétence change l'issue moyenne d'une bataille, gouverneur réduit le mécontentement, mariage
  valide/refusé, naissance déterministe, naissance historique de Charles V (1338-01-21) si Jean et Bonne mariés,
  succession salique sur mort de Philippe VI → Jean, régence, sauvegarde round-trip, déterminisme 20 tours.

## 3. API GDExtension (`CampaignSim`)
- `get_character(id) -> { id, name, epithet, sex, age, alive, faction, house, title, role, skills{command,governance,court},
  experience, skill_points, skills_learned[], traits[{id, name, category}], spouse, spouse_name, children[{id,name,age}],
  father, mother, location, army, governor_of, captive, piety, prestige }`.
- `get_faction_characters(faction) -> [ids]` (vivants, triés : dirigeant, héritier, puis par âge).
- `get_skill_tree() -> [{id, name, branch, tier, prerequisites[], cost, description}]`, `get_learnable(character) -> [ids]`.
- `get_marriage_candidates(character) -> [{id, name, age, faction}]`.
- Ordres : `learn_skill`, `assign_governor`, `assign_general` (existant), `propose_marriage`.
- `GameDataStore.get_trait(id)`, `get_skill(id)`.

## 4. Interface Godot
- Panneau « Cour » (bouton dans la barre, touche C) : liste des personnages de la faction (portrait placeholder =
  blason + initiales, nom, âge, titre, rôle actuel : général de X / gouverneur de Y / à la cour), tri, filtre.
- Fiche personnage : compétences, XP, traits avec info-bulle, famille (conjoint, enfants, parents), boutons
  « Nommer gouverneur » (choix de province contrôlée), « Donner le commandement » (armée dans la même province),
  « Marier » (liste de candidats), et l'**arbre de compétences** : trois colonnes, nœuds par tier, verrouillés/
  disponibles/appris, coût, effets ; clic = `learn_skill`.
- Panneau province : ligne « Gouverneur : X » avec bouton ; panneau armée : « Général : X (compétences) ».
- Journal : naissances, mariages, morts, successions, régences, traits acquis (couleurs dédiées).
- Smoke test : Philippe VI apprend une compétence de commandement après avoir reçu des points (forcer via
  une bataille ou via un ordre de test `debug_grant_xp` réservé au mode headless) ; nommer un gouverneur à Rouen ;
  marier deux personnages ; 40 tours : au moins une naissance et une mort dans le journal.
- Captures : `docs/img/godot-court.png`, `docs/img/godot-skill-tree.png`.

## 5. Critères de fin
- Tests Rust verts, smoke Godot vert, 60 tours en France sans crash avec dynastie vivante (Philippe VI meurt
  vers 1350 selon la table, Jean II lui succède).
