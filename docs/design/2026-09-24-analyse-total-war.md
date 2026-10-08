# Analyse comparative Total War → Cent Ans

Date : 2026-09-24. Auteur : agent d'analyse game design (session indépendante).
Objectif : rapprocher Cent Ans de Total War, visuellement et en mécaniques, en gardant les mécaniques
propres au jeu. Référence retenue par le joueur : **mélange moderne** = mécaniques de **Medieval II**
(sièges, agents, papauté, recrutement, généraux — même époque) + ergonomie et visuel des **Total War
récents** (Three Kingdoms, Thrones of Britannia, Pharaoh, Warhammer III). Priorité **équilibrée**
campagne / bataille.

**⚠️ Coordination.** `docs/archive/chantiers.md` documente une session d'orchestration **parallèle** (« session 6 »)
qui a le **même mandat** (« Rapprocher le jeu de Total War ») et prévoit d'écrire son propre plan dans
`docs/design/2026-09-24-rapprochement-total-war.md`. Ce document-ci (`2026-09-24-analyse-total-war.md`)
est une **analyse indépendante**, commandée séparément, qui ne touche aucun autre fichier. Les deux
documents vont se recouper ; à réconcilier avant de lancer des agents d'implémentation, pour ne pas
payer deux fois le même travail (le fichier `docs/archive/chantiers.md` liste déjà un lot « P1 » en cours qui touche
`no_quarter`, l'étain, l'IA de Normandie — aucun recoupement direct avec les lots proposés ici).

Hors périmètre (autres sessions, à ne pas reproposer) : refonte « colonies » (plusieurs colonies par
province, `docs/design/2026-09-24-echelle-colonies.md`, lots C1-C7) et génération des portraits.
Interactions signalées lot par lot ci-dessous.

## 1. Méthode

- Lu : `2026-09-23-cent-ans-design.md`, `2026-09-23-audit-ui-total-war.md`, `visuel-semi-realiste.md`,
  `m7-battles.md`, `m8-sieges.md`, `m9-ai.md`, `hud-campagne.md`, `battle-orders.md`,
  `2026-09-24-echelle-colonies.md`, `docs/status.md`, `docs/roadmap.md`, `docs/archive/chantiers.md`, `ui-tw.md`,
  `v4b-batailles.md`.
- Explorée l'arborescence `core/crates/{sim-campaign,sim-battle,ai}/src`, `game/scripts/{map,battle,ui}`,
  `data/schemas`.
- Regardées les captures : `docs/img/hud-campaign.png` (carte, HUD F10a intégré), `docs/img/godot-battle-f5.png`
  et `docs/img/visuel/v4b_overview.png` (bataille, avant/après finition visuelle V4b),
  `docs/img/visuel/merged_near.png` (terrain de campagne rapproché).
- Vérifié par grep l'absence de fichiers minicarte de campagne et brouillard de guerre (seule
  `game/scripts/battle/battle_minimap.gd` existe, rien côté `game/scripts/map/`) : le lot F6 prévoyait
  « minicarte cliquable » et « brouillard de guerre léger » (`docs/design/v2-finalisation.md:56`) mais a
  été transféré à la session `visual`, dont le plan (`visuel-semi-realiste.md`, lots V1-V5) ne les
  reprend pas — **perdus en route**, pas seulement en retard.
- Recherche web sur Total War (Medieval II, Three Kingdoms, Thrones of Britannia, Pharaoh, Warhammer III) :
  sources citées en fin de document.

## 2. Tableau comparatif

Gravité de l'écart : cases vides = déjà au niveau visé, pas de lot nécessaire. Coût : S (< 1 jour-agent),
M (1-3), L (> 3, à découper).

### 2.1 Campagne

| Fonctionnalité | TW (référence) | Cent Ans actuel (preuve) | Écart | Impact joueur | Coût | Domaine |
|---|---|---|---|---|---|---|
| Déplacement à points de mouvement | tous | Points d'action modulés terrain/saison/routes/rivières, Dijkstra (`movement.rs`, design §4.1) | faible | — | — | Campagne |
| Zone de contrôle | tous | Rien de comparable identifié : seul le contact déclenche une bataille (`m7-battles.md` §2) ; rien n'empêche de longer une armée ennemie sans la combattre | probable, à confirmer | armées qui se croisent sans réagir, moins tactique | M | Campagne |
| Recrutement local par bâtiments | Medieval II | Recrutement par province/classe sociale, places limitées (G1 : 2 + 1 capitale + bâtiments/techs) | faible | — | — | Campagne |
| Recrutement global instantané | Warhammer | Absent (cohérent avec la référence Medieval II) | volontaire | — | à ne pas prendre | — |
| **Agents** (prêtres, diplomates, espions, assassins, marchands) | Medieval II | Absent : diplomatie et religion gérées au niveau faction (M5), aucun personnage-agent mobile sur la carte | fort | manque la texture d'intrigue « à la Medieval II » | L | Campagne |
| Édits régionaux | Rome II+ | Seulement le taux d'imposition Bas/Normal/Haut (`m3-cities-economy.md` §1.4) | moyen | levier stratégique limité | S-M | Campagne |
| Gestion de province (classes, jauges) | tous | 4 classes × 4 jauges dynamiques, révoltes, peste, famine (`m3-cities-economy.md`) | faible, déjà riche | — | — | Campagne |
| Bâtiments en chaînes/arbres | tous | 25 bâtiments, `upgrades_from`/`required_building` (chaînes courtes) | moyen | arbres peu profonds, progression courte | M-L | Campagne — **conflit colonies** |
| Généraux : compétences/traits | Medieval II/WH | 30 compétences (3 branches), ~30 traits (M4) | faible | — | — | Campagne |
| **Arbre familial visuel** | Pharaoh/3K | Panneau Cour en liste (`court_panel.gd`), pas de graphe | moyen | dynastie moins lisible qu'en TW | S-M | Campagne/UI |
| Satisfaction publique / ordre | tous | Mécontentement par classe, révolte (`m3-cities-economy.md` §1.1) | faible | — | — | Campagne |
| Corruption régionale | WH/Rome II | Absent | hors référence Medieval II | — | à ne pas prendre | — |
| Commerce (routes, accords) | Rome II, WH | Biens par catégorie, `TradeIncome`, embargos (`m3-cities-economy.md` §1.3, `m5` §2.3) ; pas de routes visibles ni d'accords bilatéraux formels | moyen | commerce abstrait, peu visible | M | Campagne |
| Papauté / religion | Medieval II | Faveur pontificale, excommunication, Schisme 1378, Lollards/Hussites (M5) | faible, déjà au-delà de TW | — | — | Campagne |
| Auto-résolution de bataille | tous | `battle_auto.rs`, existant | faible | — | — | Campagne |
| Bataille navale | RTW+ | Transport maritime abstrait uniquement — **décision de cadrage explicite** (design §2) | volontaire | — | à ne pas prendre (hors périmètre design) | — |

### 2.2 Carte de campagne (présentation)

| Fonctionnalité | TW (référence) | Cent Ans actuel (preuve) | Écart | Impact joueur | Coût | Domaine |
|---|---|---|---|---|---|---|
| Terrain lisible, style | TW récents | Refonte V1-V6 terminée (splatmap, PBR, rivières, forêts, exagération verticale) ; capture `docs/img/visuel/merged_near.png` de bonne qualité | faible désormais | — | — | Visuel |
| **Brouillard de guerre** | tous | Prévu au plan F6, jamais livré (vérifié par grep, § 1) | total | pas d'incertitude stratégique, carte toujours entièrement lisible | M | Campagne/UI |
| **Minicarte de campagne cliquable** | tous | Seule la minicarte de bataille existe ; prévue au plan F6, jamais livrée côté campagne | total | navigation moins rapide sur une carte à 132 provinces | S-M | UI |
| UI flottante (bannières, cartouches) | TW:WH3 | Bandeau d'ost, sceau du chef, cloche de fin de saison, lettres scellées intégrés (F10a, capture `docs/img/hud-campaign.png`) | faible | — | — | UI |
| Rapport de fin de tour illustré | tous | Rapport de saison cliquable (`season_report.gd`, F3) | faible | — | — | UI |

### 2.3 Batailles

| Fonctionnalité | TW (référence) | Cent Ans actuel (preuve) | Écart | Impact joueur | Coût | Domaine |
|---|---|---|---|---|---|---|
| Caméra RTS | tous | WASD/molette/rotation (`battle_camera.gd`) ; pas de mode suivi/cinématique | faible | confort de mise en scène | S | Bataille |
| Formations | tous | Ligne/colonne/schiltron/coin, IA qui les choisit (`formation_ai.rs`) | faible | — | — | Bataille |
| Déploiement pré-bataille | Medieval II+ | Zones dorées, glisser-droit, refus explicites (F5c) | faible | — | — | Bataille |
| Moral et déroute | tous | Moral 0-100, bonus flanc/dos, ralliement (`sim-battle/src/ai.rs`, `m7`) | faible | — | — | Bataille |
| Fatigue | tous | Présente, monte en marche/charge/mêlée | faible | — | — | Bataille |
| Charge, bonus flanc/dos | tous | +50 %/+100 %, bonus de charge cavalerie | faible | — | — | Bataille |
| Terrain (pente, boue, forêt, gués) | tous | Présent, procédural par province | faible | — | — | Bataille |
| Météo (pluie, brouillard, neige) | tous | Présente avec effets sur le tir | faible, déjà au-delà de plusieurs TW | — | — | Bataille |
| Sièges (murs, brèches, échelles, tours, bélier, sortie) | Medieval II/3K | Très complet : enceinte polygonale, cheminement A*, tours qui tirent, sortie de garnison (`m8-sieges.md`) | faible, équivalent voire supérieur en simulation | — | — | Bataille |
| Cartes d'unités | tous | Compactes, groupées par « bataille », raccourcis Ctrl+1..9 (F5b, `unit_card.gd`) | faible | — | — | Bataille |
| Ordres du chef (capacités) | TW (magie/cris) | 5 ordres historiques sans magie (`battle-orders.md`) | inversé : déjà mieux adapté au cadre historique que la référence | — | — | Bataille |
| Relecture de bataille (replay) | 3K+ | Absent | total | confort, partage, pas de rejouabilité de l'analyse tactique | L | Bataille (basse priorité) |
| **Écran de fin de bataille mis en scène** | tous | Écran simple : vainqueur, pertes, bouton retour (`m7-battles.md` §4) | moyen | pas de sensation de clôture/récompense | S-M | Bataille/UI |

### 2.4 Présentation générale

| Fonctionnalité | TW (référence) | Cent Ans actuel (preuve) | Écart | Impact joueur | Coût | Domaine |
|---|---|---|---|---|---|---|
| Conseillers / narrateur récurrent | 3K (Advisor) | Aucun personnage conseiller ; couche narrative déjà riche via la Chronique (117 événements sourcés, `chronicle_window.gd`) | faible-moyen | pas de « visage » du conseil, mais contenu narratif déjà fort | M (optionnel) | Campagne |
| **Musique dynamique par intensité de bataille** | tous (WH II documenté) | `AudioDirector` change de piste par contexte (campagne/guerre/cour) mais pas par intensité de combat en cours | moyen | tension musicale statique pendant une bataille | M | Audio |
| Cinématiques d'événements | TW:WH (lore videos) | Hors budget (déjà écarté dans l'audit UI §3.3 : « portraits animés 3D hors budget ») ; la Chronique (lettrine + vignette) en tient lieu | volontaire | — | à ne pas prendre | — |

## 3. Lots proposés (priorisés valeur/coût, vagues alternant bataille et campagne)

Chaque lot respecte l'architecture : règles dans `core/` Rust, `game/` = rendu/UI/entrées, données dans
`data/` validées par schéma. Commits `wip:` fréquents (règle du projet).

### Vague 1 — fondations UI, aucun conflit colonies

**T1 — Minicarte de campagne + brouillard de guerre léger** (Campagne/UI, coût M)
- Objectif : livrer le F6 manquant (§ 2.2) : minicarte cliquable en haut à droite (relief simplifié,
  frontières, position caméra, clic/glisser = déplacement caméra) et brouillard léger (provinces non
  adjacentes à une possession ou une armée du joueur grisées, comme documenté dans `v2-finalisation.md:56`).
- Fichiers probables : nouveau `game/scripts/map/campaign_minimap.gd` + scène (s'inspirer de
  `game/scripts/battle/battle_minimap.gd`, même principe), `game/scripts/map/map_ui.gd` (accroche, cf.
  guide d'intégration `hud-campagne.md` § 3.1 pour le placement à côté de `NewsLetters`), pas de nouvelle
  règle `core/` nécessaire (le brouillard est un calcul d'affichage à partir de `get_province_state`/
  adjacence, purement Godot) — si la portée de vue doit dépendre d'une règle de jeu (ex. garnison,
  éclaireurs), passer par un getter `core/` plutôt qu'une constante GDScript.
- Critères d'acceptation : capture `docs/img/godot-campaign-minimap.png` montrant la minicarte et au moins
  une province grisée par le brouillard ; clic sur la minicarte recentre la caméra (test Godot headless) ;
  smoke `res://tests/smoke.gd` toujours vert.

**T2 — Écran de fin de bataille mis en scène** (Bataille/UI, coût S-M)
- Objectif : remplacer l'écran minimal (vainqueur/pertes/bouton retour) par un écran qui reprend la
  grammaire du reste de l'UI parchemin : écu ou portrait du vainqueur, pertes détaillées par régiment
  (déjà dans `get_outcome()` du pont, `m7-battles.md` §3), mentions notables (régiment anéanti, général
  capturé/tombé, ordre du chef utilisé).
- Fichiers probables : nouveau `game/scripts/battle/battle_result_screen.gd` + scène, appelé depuis
  `game/scripts/battle/battle_scene.gd` ; aucune nouvelle donnée `core/` a priori (tout est déjà exposé par
  `get_outcome()`).
- Critères d'acceptation : capture `docs/img/godot-battle-result.png` ; test Godot headless qui vérifie
  que l'écran affiche le nombre de pertes par camp et se ferme vers la campagne ; smoke bataille inchangé.

### Vague 2 — lisibilité stratégique et tension

**T3 — Zone de contrôle (vérification puis implémentation)** (Campagne, coût M)
- Objectif : d'abord confirmer par un test manuel/scripté qu'une armée du joueur peut aujourd'hui longer
  une armée ennemie sans la combattre ni être ralentie (aucune preuve du contraire trouvée dans
  `movement.rs`/`m7-battles.md`) ; si confirmé, ajouter une règle simple : une province contenant une
  armée ennemie active coûte un surcroît de points de mouvement (ou interdit le passage direct sans
  s'arrêter) pour une armée adverse qui la traverse sans y entrer en bataille.
- Fichiers probables : `core/crates/sim-campaign/src/movement.rs`, tests dans
  `core/crates/sim-campaign/tests/`.
- Critères d'acceptation : test Rust reproduisant le contournement avant/après ; `cargo test` vert,
  `clippy` propre ; sonde `century_probe` relancée pour vérifier l'absence de régression sur le taux de
  guerre France-Angleterre (cible ≥ 55 %, `docs/status.md` § équilibrage F4).

**T4 — Musique dynamique de bataille par intensité** (Bataille/Audio, coût M)
- Objectif : faire varier la musique en cours de bataille selon l'intensité du combat (calme avant
  contact → engagement → moment critique/déroute d'un camp), par crossfade entre les pistes déjà
  produites par `cent-ans assets audio` plutôt que de nouvelles compositions (budget 0 $ visé).
- Fichiers probables : `game/scripts/audio/` (autoload `AudioDirector`), signal d'intensité calculé côté
  Godot à partir de `get_units()`/`get_outcome()` (pas de nouvelle règle `core/`).
- Critères d'acceptation : test Godot headless qui déclenche un changement d'état (contact, déroute d'un
  camp) et vérifie le changement de piste/volume ; pas de nouveau coût cloud (0 $, `docs/budget.md`).

### Vague 3 — profondeur de gestion et de mise en scène

**T5 — Arbre familial visuel dans la Cour** (Campagne/UI, coût S-M)
- Objectif : ajouter un onglet « Arbre familial » au panneau Cour, graphe simple (parents/enfants/conjoints,
  génération courante centrée), clic = fiche personnage, à la manière du family tree de Pharaoh/Three
  Kingdoms mais dans le registre parchemin (traits à l'encre, pas d'icônes fantastiques).
- Fichiers probables : `game/scripts/ui/court_panel.gd` (+ nouvelle scène `family_tree_view.gd`/`.tscn`),
  données déjà disponibles via `get_character`/liens de parenté du pont (M4) — vérifier si un getter
  `get_family_tree(character_id)` existe déjà côté pont, sinon l'ajouter dans `godot-bridge` (lecture
  seule, pas de nouvelle règle de simulation).
- Critères d'acceptation : capture `docs/img/godot-family-tree.png` ; test Godot headless qui ouvre
  l'onglet sur une dynastie à au moins 3 générations et vérifie le nombre de nœuds affichés.

**T6 — Caméra de bataille : mode suivi/cinématique** (Bataille, coût S)
- Objectif : ajouter un raccourci qui verrouille la caméra sur un régiment ou le général sélectionné
  (vue rapprochée façon TW), bascule avec la caméra RTS libre existante.
- Fichiers probables : `game/scripts/battle/battle_camera.gd`.
- Critères d'acceptation : test Godot headless (la caméra suit la position d'un régiment sur plusieurs
  ticks) ; capture optionnelle.

### Vague 4 — contenu stratégique (attention à la refonte colonies)

**T7 — Chaînes de bâtiments approfondies et édits régionaux** (Campagne, coût M-L) — **⚠️ conflit
colonies** : la refonte colonies (`2026-09-24-echelle-colonies.md`, lots C4-C6) déplace `buildings`,
`construction`, `recruit_queue` de `ProvinceState` vers `SettlementState` et introduit
`settlement_kinds` dans `building.schema.json`. Approfondir les chaînes de bâtiments **avant** cette
migration double le travail (tout serait à réécrire pour les colonies). Recommandation : soit attendre
la fin de C4 (état colonies stabilisé), soit limiter ce lot à l'ajout d'édits régionaux (indépendants des
bâtiments) et différer l'approfondissement des chaînes à une session postérieure à C6.
- Objectif (version réduite compatible) : ajouter 2-3 édits régionaux par province (ex. « ban de
  chevauchée », « franchise commerciale », « loi martiale ») dans `data/` avec effets déjà supportés par
  `province_effects` (`Unrest`, `TaxIncome`, `RecruitCost`…), sans toucher à la structure des bâtiments.
- Fichiers probables : nouveau `data/edicts/*.json` + `data/schemas/edict.schema.json`,
  `core/crates/sim-campaign/src/buildings.rs` ou nouveau `edicts.rs`, panneau province (Godot).
- Critères d'acceptation : test Rust (édit appliqué change bien la jauge visée), test Python de schéma,
  capture du panneau province avec l'édit actif.

**T8 — Routes commerciales visibles et accords bilatéraux** (Campagne, coût M) — conflit mineur avec
colonies (le commerce pourrait un jour se rattacher aux colonies portuaires) ; sans dépendance directe
sur `ProvinceState.buildings`, donc peu de risque de retravail.
- Objectif : visualiser sur la carte les liaisons commerciales entre provinces amies/en paix qui
  échangent des catégories de biens complémentaires (déjà calculées, § 2.1), et ajouter un ordre
  diplomatique `propose_trade_agreement` (accord formel, petit bonus, distinct de l'absence d'embargo).
- Fichiers probables : `core/crates/sim-campaign/src/diplomacy.rs` (nouvel ordre), `game/scripts/map/`
  (mode de carte « Commerce », à la manière des modes N/R existants de diplomatie/religion).
- Critères d'acceptation : test Rust (accord accepté/refusé selon `evaluate`), capture du mode de carte
  commerce.

### Vague 5 — texture d'intrigue (optionnel, gros lot, à différer si le temps manque)

**T9 — Agents de campagne : espions et prédicateurs** (Campagne, coût L, **optionnel/à différer**)
- Objectif (version minimale) : réutiliser le système de personnages existant (M4) plutôt qu'un nouveau
  type d'entité : un personnage adulte sans armée ni gouvernorat peut recevoir un rôle « agent »
  (espion ou prédicateur) et une action limitée par saison : espionner une province/armée ennemie
  (révèle son contenu détaillé pendant N tours) ou prêcher (réduit l'hérésie/le mécontentement religieux
  d'une province). Pas d'assassinat ni de sabotage dans cette première étape (surface d'attaque IA/tests
  trop large pour un seul lot).
- Fichiers probables : `core/crates/sim-campaign/src/characters.rs` (nouveau rôle), nouvel ordre
  `assign_agent_action`, IA dans `ai/src/campaign.rs`.
- Critères d'acceptation : tests Rust (espionnage révèle l'état, prédication réduit l'hérésie), IA qui
  utilise l'action sans faire chuter le taux de refus d'ordres (< 20 %, cf. critère M9).
- Remarque : à ne lancer qu'après les vagues 1-4, car c'est le lot le plus coûteux et le plus risqué de
  cette liste (nouvelle mécanique de bout en bout, cœur + IA + UI).

**T10 — Relecture de bataille (replay)** (Bataille, coût L, **basse priorité**)
- Objectif : enregistrer les positions/états par tick (déjà calculés par `sim-battle`) dans un fichier
  compact, rejouable en accéléré depuis l'écran de fin de bataille (T2).
- Fichiers probables : `core/crates/sim-battle/src/` (sérialisation d'un journal de ticks, déterminisme
  déjà garanti par les tests existants), lecteur Godot dédié.
- Critères d'acceptation : test Rust (rejouer un journal produit exactement le même état final que la
  bataille d'origine, grâce au déterminisme déjà testé) ; smoke.
- Remarque : valeur secondaire (confort/partage), à ne considérer qu'une fois les vagues 1-4 livrées.

## 4. Mécaniques propres à Cent Ans à préserver

Aucun des lots ci-dessus ne doit diluer ces mécaniques, qui sont la vraie valeur différenciante du jeu
par rapport à un simple reskin de Total War :

- **Régimes alimentaires** (`data/diets`, `medicine.rs`) et système de santé/médecine historique
  (humeurs, hôtel-Dieu).
- **Monnaie** (`coinage.rs`) : dévaluation, seigneuriage, effets sur l'économie.
- **Rançons et captures** (`ransom.rs`) : capture de nobles, rançon payée/refusée, fiche personnage
  « Captif de … ».
- **Ordres de chevalerie** (`data/chivalric_orders`, `chivalry.rs`).
- **Codex historique et encyclopédie** (`data/codex`, touches K/L) : sourcé, daté, distinct de la fiction.
- **Dynasties et personnages réels** datés (M4), lois de succession différenciées (salique, préférence
  masculine, cognatique, élective).
- **Papauté et Grand Schisme** (1378-1417), hérésies Lollards/Hussites, avec dates et obédiences
  historiques.
- **Guerre de Cent Ans vivante** (F4/G2) : guerres de prétention, cobelligérance, alliances historiques
  (Auld Alliance, Bourgogne-Angleterre à Troyes), tunés par `century_probe` sur des cibles chiffrées
  (§ 2.1 du tableau, ligne « auto-résolution » exclue de tout lot).
- **Ordres du chef historiques sans magie** (`battle-orders.md`) : cri de guerre, oriflamme, pied à
  terre, pavois, ralliement — déjà l'exemple à suivre plutôt qu'un écart à combler.
- **Objectifs de victoire historiques par faction** (`data/factions/*.json` `victory`).
- **Chronique événementielle sourcée** (117 événements, chaînes historiques comme Poitiers → Brétigny →
  Troyes → Azincourt).

## 5. Risques de conflit avec la refonte colonies

- **T7** (chaînes de bâtiments) : conflit direct et fort, détaillé § Vague 4. À séquencer après C4 ou à
  réduire aux édits (indépendants des bâtiments).
- **T8** (commerce) : conflit mineur, surtout si le commerce futur se rattache aux colonies portuaires
  (`port: bool` du schéma settlement) ; le lot proposé ne touche pas `ProvinceState.buildings`, donc
  reste compatible en l'état.
- **T1, T2, T3, T4, T5, T6, T9, T10** : aucune dépendance identifiée sur `ProvinceState.buildings`,
  `garrison`, `siege`, `recruit_queue` ou `construction` (les champs que la refonte colonies retire de
  `ProvinceState`) — sans risque de conflit direct. T9 (agents) pourrait vouloir cibler une colonie
  plutôt qu'une province une fois C4 fusionné : concevoir l'action d'espionnage/prédication sur l'id de
  province en 1ʳᵉ version, migrable ensuite.
- Plus généralement : toute capture d'écran citée dans ce document ou dans les lots ci-dessus peut devenir
  obsolète après la fusion des colonies (nouveaux paliers de zoom, § 6 de `2026-09-24-echelle-colonies.md`) ;
  les recapturer après coup plutôt que de les considérer comme des critères figés.

## Sources (recherche web)

- [Medieval II: Total War — manual (Feral Interactive)](https://www.feralinteractive.com/en/manuals/medieval2/latest/steam/)
- [About Medieval II: Total War (Total War Wiki)](https://wiki.totalwar.com/w/About_Medieval_II:_Total_War.html)
- [Agents (Medieval II: Total War) — Total War Wiki](https://totalwar.fandom.com/wiki/Agents_(Medieval_II:_Total_War))
- [User interface — Total War Wiki](https://totalwar.fandom.com/wiki/User_interface)
- [Battle: What the UI is and Does — Total War Academy](https://academy.totalwar.com/battle-what-the-ui-is-and-does/)
- [Flanking — Total War: WARHAMMER Wiki](https://totalwarwarhammer.fandom.com/wiki/Flanking)
- [MTW Morale — Totalwar.org wiki](https://forums.totalwar.org/wiki/index.php/MTW_Morale)
- [Siege (Total War: Three Kingdoms) — Total War Wiki](https://totalwar.fandom.com/wiki/Siege_(Total_War:_Three_Kingdoms))
- [Broken siege mechanics — Thrones of Britannia discussion](https://steamcommunity.com/app/712100/discussions/0/2646360608735013591/)
- [Advisor (Total War: Three Kingdoms) — Total War Wiki](https://totalwar.fandom.com/wiki/Advisor_(Total_War:_Three_Kingdoms))
- [Dynasties — Total War Wiki (Pharaoh)](https://totalwar.fandom.com/wiki/Dynasties)
- [Total War: WARHAMMER 2 — how we construct and systemically use our music (Audiokinetic)](https://www.audiokinetic.com/en/blog/total-war-warhammer-2-how-we-construct-and-systemically-use-our-music/)
- [How to Play Total War: Warhammer (stances, zone de contrôle implicite) — Total War: WARHAMMER Wiki](https://totalwarwarhammer.fandom.com/wiki/How_to_Play_Total_War:_Warhammer)
