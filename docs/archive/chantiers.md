# Archive des chantiers clos

Notes `docs/wip/<x>.md` closes, condensées puis supprimées du dépôt (ménage SC, 2026-10-08). Chaque entrée garde le nom de fichier d'origine en entête (`grep '^### <x>.md'`). Le texte complet reste dans `git log -- docs/wip/<x>.md`. Les restes des chantiers « avec restes » sont recopiés dans `docs/wip/restes.md`.

## Grappe `battle`

### battle-orders.md — WIP — Ordres du chef en bataille (F10b) (2026-09-23)
- Branche : `feat/battle-orders`. Spécification : `docs/design/battle-orders.md`.
- Données `data/battle_orders/*.json` (5 ordres) + `data/schemas/battle_order.schema.json` + test pytest.
- `data-model` : `entities/battle_order.rs`, `GameData::battle_orders` (dossier facultatif).

## Grappe `f`

### f1-effects.md — Lot F1 « Règles inertes » — état (2026-09-23)
- Branche : `worktree-agent-ae66d663ee24e14ab`. Tests : `core/crates/sim-campaign/tests/f1_effects.rs`.
- 1. [x] Bâtiments : `Garrison`, `RecruitCost`, `Supply`, ciblage par classe (et par catégorie d'unité).
- 2. [x] Technologies : `army_upkeep`, `army_experience`, `recruit_cost`, `movement`, `production`,

### f2-icons.md — WIP F2 — Icônes et infobulles (2026-09-23) [restes]
- Branche : `worktree-agent-af998097891c4cf5c`. Plan : `docs/design/v2-finalisation.md` (lot F2).
- Outil Python `tools/cent_ans_tools/icons.py` + `icons_catalog.py`, CLI `cent-ans assets icons`, tests `tools/tests/test_icons.py` (9).
- 144 identifiants → 106 SVG dans `game/assets/icons/` + `icons.json` + `.import` (mipmaps).

### f3-flow.md — Lot F3 « Écrans et flux » — suivi (2026-09-23)
- Branche : `worktree-agent-a6a48de5c2de758a2`.
- 1. [x] Illustration du menu (`menu_art.py`, `cent-ans assets menu-art`) + menu de départ illustré.
- 2. [x] Écran de chargement (`loading_screen.gd`).

### f4-war.md — Lot F4 « Guerre de Cent Ans vivante » — état (2026-09-24) [restes]
- Branche : `worktree-agent-a5777744f85f20632`. Sonde : `cargo run --release -p ai --example century_probe`
- (5 graines × 464 tours en parallèle, tableau de synthèse ; `VERBOSE=1` pour le détail).
- Tests : `core/crates/ai/tests/f4_war.rs` (7), `core/crates/sim-campaign/tests/f4_succession.rs` (4).

### f5a-battle-sim.md — F5a — Simulation de bataille, finition (2026-09-24) [restes]
- Branche : `worktree-agent-af73653c66d612f3a`. Spécification : `docs/design/m7-battles.md` § F5,
- `docs/design/m8-sieges.md` § F5.
- 1. [x] Collisions amies (`src/sim/separation.rs`).
- Restes : 5. [ ] Renforts échelonnés au-delà de 20 régiments : **non fait**.

### f5b-battle-hud.md — F5b — HUD de bataille (audit UI § 3.2) (2026-09-24)
- Branche : `worktree-agent-a9f2761a254f30730`.
- 1. [x] `unit_card.gd` : cartes moitié moins larges (icône de classe, effectif, barres fines, infobulle formation, noms coupés sur un mot entier).
- 2. [x] `battle_groups.gd` : cartes groupées par bataille (avant-garde / corps / arrière-garde) ; Ctrl+chiffre enregistre, chiffre rappelle (raccourcis à vérifier).

### f5c-deploy-ui.md — F5c — Déploiement et maisons de siège dans Godot (2026-09-24)
- État : terminé (points 1 à 4).
- `game/scripts/battle/deployment_controller.gd`, `deployment_zone.gd` : phase de déploiement.
- `battle_scene.gd` : section « F5c » en fin de fichier, `--deploy-shot`, Entrée, clic droit routé.

### f5d-battle-fixes.md — F5d — Simulation de bataille : correctifs (2026-09-24) [restes]
- Branche : `worktree-agent-ae415cdff5eabc27d`. Périmètre : `core/crates/sim-battle`, ajouts dans
- `core/crates/godot-bridge/src/battle_sim.rs`. Détail : `m7-battles.md` et `m8-sieges.md` § F5d.
- 1. [x] Bataille de démo (France 1337, graine 1337) : contact à 75 s, personne dans l'eau.

### f7-events.md — Lot F7b « Chronique enrichie » — état (2026-09-24) [restes]
- Branche : `worktree-agent-a65b4684eae74eccc`. Tests : `core/crates/sim-campaign/tests/f7_events.rs`.
- 50 événements. Déjà présents (pas de doublon) : L'Écluse, Nicopolis, du Guesclin connétable, Cabochiens,
- mort du Prince Noir, assassinat de Louis d'Orléans (`evt_armagnacs_bourguignons`), Troyes, Jeanne d'Arc,

## Grappe `h`

### h1-audit.md — H1 — Audit historique (état) (2026-09-23)
- Branche : worktree-agent-a62a0232514bd83d9
- État : **terminé**. Toutes les données relues ; corrections appliquées dans `data/` ; rapport `docs/histoire/audit-2026-09-23.md` rédigé.
- Tests : `cd core && cargo test` passe (dont `data-model/tests/real_data.rs`).

### h2-codex.md — WIP — H2 Codex et bulles imbriquées (2026-09-23)
- Conception : `docs/design/2026-09-23-histoire-et-savoir.md` §1. Branche : worktree agent H2.
- Schéma `data/schemas/codex.schema.json` + `codex_id` dans `common.schema.json`
- 20 fiches d'amorce `data/codex/cdx_*.json` + `data/codex/_todo.md` (21 fiches de la vague 2)

### h3-h4-table-medecine.md — WIP — H3 « La Table » + H4 « Médecine » (règles, données, pont) (2026-09-23) [restes]
- Conception : `docs/design/2026-09-23-histoire-et-savoir.md` § 2-3. API : `docs/design/h3-h4-api.md`.
- 1. [x] Squelette : `DietId`, `Diet` (data-model), schéma `diet.schema.json`, `TechBranch::Medicine`,
- `EffectKind::{PlagueResistance, WoundRecovery, DietHealth}`, `Technology::herbs`,

### h7-events.md — H7 — événements historiques manquants (audit § 7) (2026-09-23)
- Générateur : `scratchpad/gen_h7.py` (hors dépôt) ; les JSON sont dans `data/events/`.
- Lot 1 (1337-1340) : cadzand, artevelde, siege_de_dunbar, sac_de_southampton, vicariat_imperial, roi_de_france_a_gand, salado, treve_d_esplechin
- Lot 2 (1341-1357) : mort_jean_iii, arret_de_conflans, morlaix, auberoche, nevilles_cross, traite_de_berwick, ordre_de_la_jarretiere, winchelsea, combat_des_trente, ordre_de_l_etoile

### h11-ui-monnaie-rancons.md — WIP — H11 interface Monnaie, Rançons, Ordre de chevalerie, Encyclopédie ↔ Codex (2026-09-24)
- Conception : `docs/design/2026-09-23-histoire-et-savoir.md` §5.1-5.2 ; API : `docs/design/h5-h6-api.md`.
- Modèle : `docs/wip/h9-ui-table-medecine.md` (`table_section.gd`).
- `game/scripts/ui/coinage_section.gd` : niveau, `price_level` + jauge, seigneuriage/refonte

### h12-codex-evenements.md — H12 — Codex (héraldique, calendrier, vie quotidienne) + événements pédagogiques (2026-09-24)
- A. 27 fiches Codex écrites
- B. 14 événements pédagogiques écrits
- Tests : pytest complet (87 passed), cargo test -p data-model (ok, aucun avertissement sur les nouveaux événements)

### h5-h6-monnaie-chevalerie.md — WIP — H5 « Monnaie » + H6 « Chevalerie, rançons, ordres » (règles, données, pont) (2026-09-24) [restes]
- Conception : `docs/design/2026-09-23-histoire-et-savoir.md` § 5.1-5.2. API : `docs/design/h5-h6-api.md`.
- Branche : `worktree-agent-a5c437f998e41d92b`.
- 1. [x] Squelette : `sim-campaign::{coinage, ransom, chivalry}` (non branchés), champs d'état

### h9-ui-table-medecine.md — WIP — H9 interface « La Table » et Médecine (2026-09-24)
- Conception : `docs/design/2026-09-23-histoire-et-savoir.md` §2.3, §3 ; API : `docs/design/h3-h4-api.md`.
- `game/scripts/ui/table_section.gd` : régime actuel, description (mots du Codex cliquables),
- bandeau Carême (`is_lent`), sélecteur (`get_diet_options`, infobulle `RichTooltip.diet`),

### h10-codex-table-medecine.md — H10 — Codex : Table, cuisine, plantes, médecine (2026-09-28)
- Branche : `worktree-agent-a038490f6abdd9c53`. Agent historien (alimentation, médecine).
- 22 ids restants de `data/codex/_diet_links.md` (`cdx_viandier`, `cdx_gabelle` existent déjà).
- 22 ids de `data/codex/_herb_links.md`.

## Grappe `hud`

### hud-campagne.md — WIP — HUD de campagne (lot F10a, composants) (2026-09-23)
- Branche : `worktree-agent-a74b133436188f172`.
- Squelette : 4 scènes `game/scenes/ui/{army_strip,general_seal,end_turn_cluster,news_letters}.tscn`, scripts, `hud_style.gd`.
- ArmyStrip (cartes pleines ou compactes sur deux rangs, noms sans coupure de mot, multisélection, séparer)

### hud-integration.md — WIP — Intégration du HUD de campagne (F10b) (2026-09-24) [restes]
- Branche : `worktree-agent-ac6575fb236a85869` (depuis `lot-a`).
- 1. [x] Squelette `HudController` (`game/scripts/map/hud_controller.gd`).
- 2. [x] `MapUI` : nœuds `ArmyStrip`, `GeneralSeal`, `EndTurnCluster`, `NewsLetters` ; `layout_hud()` ; accesseurs

## Grappe `v`

### v2-terrain.md — V2 — Terrain de campagne semi-réaliste (branche `visual-v2`) (2026-09-23)
- Plan : `docs/design/visuel-semi-realiste.md` (lot V2), ADR 0004. **État : terminé**, à fusionner par
- l'orchestrateur. Captures : `docs/img/visuel/v2_*.png` (`v2_campaign_far`/`v2_campaign_near` = mêmes
- cadrages que les `before_*` ; `v2_map_far`, `v2_map_mid`, `v2_alps`, `v2_alps_far`, `v2_coast` sans sélection).
- Réf. : ADR 0004

### v2b-carte.md — V2b — Finitions de la carte de campagne semi-réaliste (branche `visual-v2b`) (2026-09-24)
- Depuis `visual` (V1-V4 fusionnés). Captures de travail dans le scratchpad, finales dans
- `docs/img/visuel/v2b_*.png`.
- 1. Parcellaire commun terrain/haies : grille biaisée déformée (même fonction en GDScript et en shader),

### v3-vegetation.md — WIP — lot V3 « Végétation, villes et marqueurs de la carte de campagne » (2026-09-24)
- Branche `visual-v3` (depuis `visual`, V1 + V2 fusionnés). **État : terminé**, à fusionner par l'orchestrateur.
- Captures : `docs/img/visuel/v3_far.png`, `v3_mid.png`, `v3_forest.png`, `v3_city.png`, `v3_army.png`,
- `v3_army_states.png` (au repos sélectionnée / en marche « » » / siège / à bord).

### v4-batailles.md — Lot V4 — Batailles semi-réalistes (branche `visual-v4`) (2026-09-24)
- Textures Poly Haven CC0 + procédurales dans `game/assets/textures/battle/` (README, `build_textures.py`)
- Ciel, lumière, météo (`battle.tscn`, `battle_sky.gdshader`, `battle_atmosphere.gd`, particules GPU) — premier jet
- Sol texturé (splatmap, `battle_ground.gdshader`), anneaux de collines — premier jet

### v4b-batailles.md — Lot V4b — Finitions des batailles semi-réalistes (branche `visual-v4b`) (2026-09-24)
- Banc d'essai : médiane des durées d'image, primitives et appels de dessin en plus de la moyenne
- (Metal ne fournit pas le temps GPU au script)
- Arbres par tuiles de 160 m (écartés hors champ et hors ombres) + maillage allégé au-delà de 260 m,

### v6-perf.md — V6 — Performance du rendu (branche `visual-v6`) (2026-09-24)
- Depuis `visual` (999ea1a, « V2b merged, main integrated »). Machine partagée (M4 Pro), mesures
- bruitées (±30 %), chaque mesure répétée au moins deux fois.
- Profilage temporaire (chronomètres par étape, retiré avant commit) sur `vegetation_bench.gd` :

### v1-correctifs-visuels.md — V1 — correctifs visuels rapides (audit A1) (2026-09-25) [restes]
- Source : `docs/audit/a1-visuel.md`. Captures avant/après : `docs/audit/captures/v1/`.
- A1-02 zone de déploiement discrète : `game/shaders/deployment_zone.gdshader` (liseré à largeur écran minimale, halo, hachures en bande de 22 m, fondu 140→700 m de caméra) ; captures `02_*`
- A1-03 brouillard non noir : `terrain.gdshader` (désaturation, brume parchemin, nuages bas fbm animés par TIME, bord fondu 3 px sur les frontières vu/voilé), `campaign_minimap.gdshader` (voile clair) ; captures `03_*`

### v2-soldats-animes.md — Lot V2 — Soldats et chevaux animés (audit A1-18, A1-07, A1-01) (2026-09-25)
- Branche `worktree-agent-aa60537d86d664cc6` (a fusionné `main` puis `integration/night` pour les
- assets D0 : Quaternius dans `game/assets/third_party/`). ADR : `docs/decisions/0014-figurines-skinnees-texture-os.md`.
- Figurines skinnées avec animations cuites en texture d'os, 9 figurines (6 à pied, 3 montées),
- Réf. : ADR 0014

### v4-fleuves-forets.md — V4 — Fleuves, ponts et forêts de la carte de campagne (lots A1-11, A1-10) (2026-09-25)
- Branche `worktree-agent-ae5685646cc8b59ac` (fusionne `main` jusqu'à 113ce3e2 et la branche L2 : L1, L2, R1, CV1, CV2, M5b…).
- Rendu seulement. Captures : `docs/audit/captures/v4/` (`avant_*` sur main 356a1ad, `apres_*`).
- Banc : `godot --rendering-driver vulkan --disable-vsync --path game --script res://tests/v4_map_bench.gd`
- Réf. : commits 113ce3e2, a8e1cc7a

## Grappe `a`

### a1-audit-visuel.md — WIP — audit A1 (visuel, 3D, animation), session de nuit 7 (2026-09-24)
- État : terminé. Rapport `docs/audit/a1-visuel.md`, captures `docs/audit/captures/a1/` (30 PNG).
- Prochaine étape : aucune pour A1 ; l'orchestrateur choisit les lots (§ 4 du rapport).

### a2-audit-mecaniques.md — WIP — audit A2 mécaniques et équilibre (2026-09-24)
- État : **terminé**. Livrable `docs/audit/a2-mecaniques.md` (§ 0-6 : chiffres de 8 graines × 200 tours et 4 × 464 tours, matrice d'auto-résolution, comparaison avec la bataille 3D, boucles TW, lots O1, E1-E9, N1-N8). Sources de la…
- Prochaine étape (hors audit) : lancer O1 (sonde dans le dépôt), puis N1 + E1.

## Grappe `b`

### b1-maillages.md — Lot B1 — Maillages des soldats et des chevaux (branche `worktree-agent-a1fa8fd1c52def061`) (2026-09-24)
- Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (lot B1).
- Modélisation en script Blender reproductible : `tools/blender/battle_figures.py`
- (`blender --background --python tools/blender/battle_figures.py`), qui exporte un `.glb` par
- Réf. : ADR 0006

### b2-bannieres-vignettes.md — B2 — Bannières flottantes, cartes-vignettes, écran de fin (lot T2) (2026-09-24)
- Branche : `worktree-agent-a0c31d412af478ab6`. Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (B2).
- Ne pas toucher : `battle_meshes.gd`, `battle_soldiers.gd`, `battle_soldier.gdshader` (lot B1).
- 1. [x] `battle_unit_markers.gd` : repères 2D projetés (Control unique plein écran, `_has_point` sur

### b3-musique-camera.md — B3 — musique dynamique de bataille (T4) + caméra de suivi (T6) (2026-09-24)
- Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (lot B3), spéc T4/T6 dans
- `docs/design/2026-09-24-analyse-total-war.md`.
- Squelette + exploration (`AudioDirector`, `BattleScene`, `BattleCamera`, `get_units()`,

### b4-effets-animations.md — Lot B4 — Effets et animations de bataille (fusionné dans main, `2a485a5`) (2026-09-24)
- Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (lot B4). Suite de B1 (`docs/wip/b1-maillages.md`).
- `cd core && cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test` : vert
- (au commit 7eacc6c ; le Rust n'a plus changé depuis).
- Réf. : ADR 0006

### b5-champs-de-bataille.md — Lot B5 — Champs de bataille tirés de la campagne (2026-09-24)
- Plan : `docs/design/2026-09-24-rapprochement-total-war.md`. Branche du worktree B5 (non fusionnée).
- Le cœur tirait déjà relief / forêts / boue / rivière selon le terrain de la province (7 terrains) et
- la météo selon la saison ; mais ni côte, ni village, ni haies, ni mares, ni sol de saison (neige au

### b6-ia-terrain.md — Lot B6 — IA tactique et terrain de site (2026-09-24)
- Plan : `docs/design/2026-09-24-rapprochement-total-war.md`, suivi `docs/wip/tw.md`. Branche du worktree B6
- (`worktree-agent-a87ff8639d57202f0`), non fusionnée.
- IA, position défensive (`core/crates/sim-battle/src/ai.rs`, `defensive_cover`, `Cover`) : un camp

### b7-finitions-bataille.md — Lot B7 — Finitions visuelles de bataille (terminé, non fusionné) (2026-09-24)
- Plan : `docs/design/2026-09-24-rapprochement-total-war.md`, suivi `docs/wip/tw.md`. Suites de B1/B2/B4/B5.
- Godot uniquement (`game/`), aucune règle.
- Cause : le fanion est modélisé perpendiculaire à la hampe (flottant vers l'arrière, lance droite) et

### b8-suites-bataille.md — Lot B8 — suites de bataille (relevées par B6/B7) (2026-09-24) [restes]
- Plan : `docs/design/2026-09-24-rapprochement-total-war.md`, suivi `docs/wip/tw.md`. Suites de B4/B6/B7.
- Point 1 (IA Rust, `sim-battle`) terminé et testé ; points 2-4 (Godot : minicarte, boue/gué,
- infobulle) et les captures/mesures n'ont pas été traités dans cette session — voir « Points

### b1-bulles-infra.md — WIP — B1 Infra bulles (T universel) (2026-09-25)
- Branche : `feat/b1-bulles-infra` (worktree agent). Spec : `docs/design/2026-09-25-bulles-partout.md`.
- T universel (`CodexBubbles.pin_current`) : bulle non épinglée (ou en attente) > infobulle riche > infobulle simple du contrôle survolé (après le délai d'infobulle)
- Chaîne parent → enfant (méta `parent`, `parent_of`) : épingler épingle les ancêtres, détacher détache les descendantes, une bulle retirée rattache ses filles à sa parente

### b2-mecaniques-campagne.md — WIP — B2 Mécaniques de campagne (fiches Codex `cdx_jeu_*`) (2026-09-25) [restes]
- Spec : `docs/design/2026-09-25-bulles-partout.md` (lot B2). Branche : `worktree-agent-a1391ebbb72a033ca`.
- Collecte des chiffres (code `core/crates/sim-campaign`, `data/`)
- `gameplay` ajouté aux fiches existantes (zone de contrôle, chevauchée, rançon, mutations, taille, gabelle, aides, Carême, jours maigres, Peste noire, places fortes, ponts et gués) + see_also vers les fiches jeu

### b3-mecaniques-bataille.md — WIP — B3 Mécaniques de bataille, siège et guerre navale (Codex « Le jeu ») (2026-09-25)
- Conception : `docs/design/2026-09-25-bulles-partout.md` (lot B3). Suivi général : `docs/wip/bulles-partout.md`.
- Fiches `category: "mecanique"` (ids `cdx_jeu_…`) sur la bataille 3D, l'auto-résolution, les sièges
- (campagne et assaut 3D), les incendies, les engins et la guerre navale ; une fiche par ordre du chef

### b4-batiments.md — WIP — B4 Bâtiments et ressources (bulles partout) (2026-09-25)
- Branche : `b4-batiments`. Conception : `docs/design/2026-09-25-bulles-partout.md`.
- 30 bâtiments, 10 ressources : chacun a une fiche Codex avec `entity` et `gameplay` (chiffres tirés
- de `data/buildings/`, `data/resources/` et de `core/crates/sim-campaign/src` : buildings.rs,

### b6-audit.md — B6 — Audit historique des ajouts récents (depuis le 24/09/2026) (2026-09-25) [restes]
- Branche : `b6-audit-historique`. Spec : `docs/design/2026-09-25-bulles-partout.md` (lot B6).
- Rapport : `docs/histoire/audit-2026-09-25.md`.
- Colonies `data/settlements/` (133 fichiers, 577 colonies), 131 corrections

### b7a-economy-order.md — B7a — incohérences code ↔ interface : économie et ordre (2026-09-25) [restes]
- Branche `b7a-economy-order` (depuis main 04657c09). Liste d'origine : `docs/wip/bulles-partout.md`, « Incohérences code ↔ interface relevées par B2 ».
- 1. **Cour / opulence** : on garde le code (20 % de l'excédent au-delà de 6 saisons de revenu, réglé en F4 et vérifié par `m10_balance`). Les constantes d'administration, d'opulence et de banqueroute passent dans `data/rules/econo…
- 2. **Dette** : pas de débandade (le design m2 § 5 dit « trésor négatif → moral −10 ») : textes alignés sur le code.
- Réf. : commits 04657c09, 79e0c7c4

### b7b-unread-data.md — B7b — données jamais lues par le code (2026-09-25)
- Branche : `b7b-unread-data`, rebasée sur main (dc2360c6). Contexte : `docs/wip/bulles-partout.md` (B2, vague 3).
- 1. Piété des édits (Paix de Dieu +2, Carême strict +4) : chiffre annuel versé au souverain chaque hiver, meilleure province tenue entière seulement (comme la table H3, pas de cumul par province), en plus de la piété des bâtiments…
- 2. `construction_speed` (trait Bâtisseur +15 %, compétences Bâtisseur +20 % et Urbaniste +15 %, et bâtiments/édits éventuels, aucun aujourd'hui) : durée = arrondi(base × 100 / (100 + %)), au moins 1 tour, fixée à la commande ; %…
- Réf. : commits dc2360c6

### b7c-buildings.md — B7c — bâtiments : incohérences données ↔ code ↔ interface (2026-09-25)
- Branche `b7c-buildings` (worktree agent). Ne pas fusionner dans main (orchestrateur bulles vague 3).
- 1. **Améliorations sans régression** : une amélioration porte le total de sa chaîne. Le chargeur
- (`data-model`, `check_buildings`) refuse une amélioration qui perd ou affaiblit un effet du bâtiment
- Réf. : ADR 0053

### b8-homonymes.md — WIP — B8 Auto-lien sans homonymes (2026-09-25)
- Branche : `feat/b8-homonymes` (worktree agent). Contexte : `docs/wip/bulles-partout.md`, vague 2.
- Alias le plus long prioritaire (tri déjà présent dans `CodexStore.alias_regex`) ; limites
- de mot : le tiret entre deux mots les soude (« Saint-Omer » ne lie pas « Omer »,

### b8b-visuels-bataille.md — Lot B8b — suites visuelles de bataille (minicarte, boue, gué, pastilles) — terminé, non fusionné (2026-09-25)
- Plan : `docs/design/2026-09-24-rapprochement-total-war.md`, suivi `docs/wip/tw.md`. Partie visuelle
- (Godot uniquement) des points 2-4 relevés par `docs/wip/b8-suites-bataille.md` (partie IA, branche
- `worktree-agent-a7845880a6ea35c48`, fusionnée à part). Aucun fichier `core/` touché.

### b9a-fiches.md — WIP — B9a : fiches Codex, sujets 1 à 9 (audit 2026-09-25 § 10) (2026-09-25) [restes]
- Branche : `b9a-fiches` (partie de `integration/historien`).
- Lot terminé : validateur vert (397 fiches), `pytest tools/tests/test_codex.py` vert. Reste : fusion dans `integration/historien` (conflit probable avec B9b dans l'audit § 10, à résoudre à la main).
- Pas de build (disque plein) : seul le validateur Python.

### b9b-fiches.md — B9b — fiches Codex, sujets 10 à 18 de l'audit § 10 (2026-09-25)
- Branche : `worktree-agent-afbe783dbb349d1a1` (partie de `main` 580fc208).
- 10 `cdx_marguerite_maultasch` (entité `prov_tirol`)
- 11 `cdx_adolphe_de_la_marck` (entité `prov_liege`)
- Réf. : commits 580fc208

## Grappe `c`

### c1-minicarte-brouillard.md — WIP — C1 minicarte de campagne + brouillard de guerre léger (2026-09-24) [restes]
- Spéc : lot T1 de `docs/design/2026-09-24-analyse-total-war.md` ; plan `2026-09-24-rapprochement-total-war.md` (C1).
- Règle de vue côté Rust : `CampaignState::visible_provinces(data, faction)` (`core/crates/sim-campaign/src/vision.rs`),
- données `data/rules/vision.json` (schéma `vision_rules.schema.json`, chargé dans `GameData.vision_rules`),

### c1-settlements-skeleton.md — C1 — squelette des colonies (2026-09-24) [restes]
- Spec : `docs/design/2026-09-24-echelle-colonies.md` § 8 (lot C1). ADR 0005.
- `data-model` : `SettlementId` (`set_`), `SettlementKind`, `Settlement`, `SettlementRules`,
- `SettlementGraph`/`SettlementEdge` (`entities/settlement.rs`), chargement + repli + validation
- Réf. : ADR 0005
- Restes : Smoke Godot : échec **préexistant** (reproduit sur 64dec0b sans ce lot) : / La garnison de la cité est vide dans `SettlementState` (la garnison de province reste la vraie / Règle « au moins un port par province côtière » (§ 3.1) non vérifiée par le test Python (non

### c2-zone-controle.md — WIP — C2 zone de contrôle (rapprochement Total War) (2026-09-24)
- **Suspendu : remplacé par M2 mouvement libre.** Une autre session lance M2
- (`docs/design/2026-09-24-mouvement-libre.md`), qui réécrit `movement.rs`
- avec sa propre zone de contrôle (8 km, grille A*). Ne rien pousser de plus

### c2a-settlements-france.md — C2a : colonies de 1337 — France (7 régions, 41 provinces) (2026-09-24) [restes]
- Spec : `docs/design/2026-09-24-echelle-colonies.md` § 3.1. Schéma : `data/schemas/settlement.schema.json`.
- Régions couvertes : `france_nord`, `france_centre`, `france_est`, `france_ouest`, `aquitaine`, `languedoc`, `provence_alpes`.
- Validation : `uv run --project tools python <script ad hoc>` (jsonschema + registry common.schema.json, règles métier :

### c2b-settlements-north.md — C2b — colonies des îles Britanniques, Pays-Bas, Empire, Scandinavie (2026-09-24) [restes]
- Tâche : `docs/design/2026-09-24-echelle-colonies.md` § 3.1. 56 provinces (régions
- `angleterre_*`, `galles`, `ecosse`, `irlande`, `pays_bas`, `empire_*`, `scandinavie`).
- 56/56 provinces écrites, 227 colonies (56 city + 89 town + 38 castle + 34 abbey +

### c2c-settlements-south.md — C2c — colonies d'Ibérie et d'Italie (2026-09-24) [restes]
- Tâche : `docs/design/2026-09-24-echelle-colonies.md` § 3.1. 35 provinces (régions
- `iberie_centre`, `iberie_nord`, `iberie_sud`, `aragon`, `portugal`, `italie_centre`,
- `italie_nord`, `italie_sud`).

### c3-geo-settlements.md — Lot C3 : pipeline géo des colonies (2026-09-24) [restes]
- Spec : `docs/design/2026-09-24-echelle-colonies.md` § 3.3, 3.4, 4.1, 4.4, 5. Doc : `docs/geo.md`.
- `settlements` : projection, contrôle province, graphe, `settlements_px.json`, aperçu
- `roads` : Itiner-e (Zenodo 17122148, gpkg sans compte) + routes calculées hors limes

### c3-personnages.md — WIP — C3 personnages à la Total War (arbre familial, fiche de général) (2026-09-24)
- Spéc : lot C3 de `docs/design/2026-09-24-rapprochement-total-war.md` (T5 de `2026-09-24-analyse-total-war.md`).
- Pont : `CampaignSim.get_family_tree(character, up, down) -> {root, ruler, heir, nodes}`
- (`core/crates/godot-bridge/src/campaign_sim_family.rs`, lecture seule, aucune règle). Nœuds :

### c4-core-settlements.md — C4 — refonte du cœur (colonies) (2026-09-24)
- Spec : `docs/design/2026-09-24-echelle-colonies.md` § 2, 4.2-4.6. ADR 0005. Suit C1
- (`docs/wip/c1-settlements-skeleton.md`).
- Données : `settlement_kinds` des bâtiments (schéma + 24 fichiers) ; dans
- Réf. : ADR 0005

### c5-settlements-ui.md — Lot C5 : pont et interface des colonies (2026-09-24)
- Spec : `docs/design/2026-09-24-echelle-colonies.md` § 4.6 et § 6. Suit C4
- (`docs/wip/c4-core-settlements.md`) et C6 (`docs/wip/c6-zoom-tiers.md`).
- Branche : `worktree-agent-ac38aa74f75611f21` (partie de `main` `cda86a9`).

### c6-agents.md — WIP C6 — agents de campagne (2026-09-24) [restes]
- Branche : `worktree-agent-a64ab636a1a413848`. Conception : `docs/design/2026-09-24-agents.md`, ADR 0009.
- (a) conception, ADR, données `data/rules/agents.json` + schéma + test Python, entité `data-model::AgentRules`
- (a)+(b) `sim-campaign/src/agents.rs` complet (ordres, actions, contre-espionnage, vision, IA `plan_agents` appelée par `ai::plan_turn`) + tests `sim-campaign/tests/c6_agents.rs` (17) et `ai/tests/c6_agents_ai.rs`
- Réf. : ADR 0009

### c6-zoom-tiers.md — Lot C6 : rendu de la carte par paliers de zoom (2026-09-24)
- Spec : `docs/design/2026-09-24-echelle-colonies.md` § 6. Branche : `worktree-agent-ab78e060f95eaa4e6`.
- Documentation : `docs/godot-map.md` § « Paliers de zoom, colonies, hameaux et routes (lot C6) ».
- `ZoomTiers` (`game/scripts/map/zoom_tiers.gd`, `game/resources/zoom_tiers.tres`)

### c7-suite-general.md — WIP — C7 suite du général (« retinue » à la Medieval II), année de décès, mise en page de la fiche (2026-09-24)
- Spéc : demande orchestrateur (lot C7 du plan `docs/design/2026-09-24-rapprochement-total-war.md`, suites de C3).
- Branche : `worktree-agent-a29adc13935454195`.
- Données `data/retinue.json` (15 compagnons, plafond 8) + schéma `data/schemas/retinue.schema.json`

### c7a-settlements-balance.md — C7a — repli du perdant, IA sur les colonies, équilibrage 50 tours (2026-09-24)
- Spec : `docs/design/2026-09-24-echelle-colonies.md` (§ 4.3-4.5, § 7), suite de C4
- (`docs/wip/c4-core-settlements.md`). Branche : `worktree-agent-a2e37f1c8b64c0bf2` (depuis `main` cda86a9).
- Périmètre : `core/` et `data/` uniquement.

### c7b-settlements-render.md — Lot C7b : rendu des colonies (arbres, routes, chemins, panneaux) (2026-09-24)
- Spec : `docs/design/2026-09-24-echelle-colonies.md` § 6. Suit C5 (`docs/wip/c5-settlements-ui.md`)
- et C6 (`docs/wip/c6-zoom-tiers.md`). Branche : `worktree-agent-aa38756fc56f579e1` (partie de `main` `ff21e60`).
- Périmètre : rendu Godot (et pipeline géo si besoin) ; C7a touche `core/` en parallèle.

### c7c-settlements-docs.md — C7c — documentation des colonies (manuel, codex) (2026-09-24)
- Spec : `docs/design/2026-09-24-echelle-colonies.md`. ADR 0005. Suit C4 (`docs/wip/c4-core-settlements.md`),
- C5 (`docs/wip/c5-settlements-ui.md`), C7a (`docs/wip/c7a-settlements-balance.md`).
- Périmètre : `docs/manuel.md`, `data/codex/`, `game/scripts/ui/encyclopedia.gd` (fiches
- Réf. : ADR 0005

### c7d-garrison-button.md — Lot C7d : bouton « Laisser en garnison » (2026-09-24)
- Spec : ordre `Order::GarrisonUnits` (C7a, `core/crates/sim-campaign/src/orders.rs`),
- demande C5/C7a « pas encore de bouton dans l'UI ». Branche :
- `worktree-agent-af13b589dd9cd2655` (main `0a7bc32` fusionné).

### c4-edits-chaines.md — WIP — lot C4 (TW) : édits régionaux et chaînes de bâtiments (2026-09-25) [restes]
- Mandat : `docs/design/2026-09-24-rapprochement-total-war.md` (C4), analyse TW §2.1
- (« Bâtiments en chaînes/arbres », « Édits régionaux »). Démarré après la fusion
- de la refonte colonies (`docs/wip/colonies.md`, ADR 0005).
- Réf. : ADR 0005, 0011
- Restes : Données de colonies d'avant C4 : plusieurs paliers d'une même chaîne restent / Pas de condition de taille de colonie au-delà de `settlement_kinds` et des / L'arbre de construction reste une liste triée (rang affiché, prérequis en

### c5-commerce.md — C5 — Routes commerciales et accords (rapprochement Total War) (2026-09-25)
- Squelette data-model posé : `data/economy/trade.json` (hubs + routes), chargé
- dans `GameData::trade` (`core/crates/data-model/src/entities/trade.rs`,
- Rust fait et testé (`core/crates/sim-campaign/tests/c5_trade.rs`, 8 tests
- Réf. : ADR 0012 ; commits 6ebe9658, 58733bb2

## Grappe `colonies`

### colonies.md — Orchestration : refonte « colonies » (échelle Total War) (2026-09-24)
- Spec : `docs/design/2026-09-24-echelle-colonies.md`. ADR : `docs/decisions/0005-settlements-within-provinces.md`.
- Le joueur a validé la spec et autorise toutes les décisions sans demander (2026-09-24).
- La session 6 (`docs/wip/tw.md`, plan `docs/design/2026-09-24-rapprochement-total-war.md`) rapproche aussi le jeu de Total War, colonies exclues de son périmètre. Ses lots s'appellent aussi « C1 »… (minicarte, brouillard) : sans r…
- Réf. : ADR 0005

### colonies-liste.md — Liste des colonies (touche B) — suivi (2026-09-27)
- Spec : `docs/superpowers/specs/2026-09-27-liste-colonies-design.md`.
- Plan : `docs/superpowers/plans/2026-09-27-liste-colonies.md`.
- `feat/holdings-core` : cœur (`sim-campaign::holdings::holdings_overview`, 7 tests
- Réf. : ADR 0097 ; commits 8c926169, bc53e337

## Grappe `finalisation`

### finalisation.md — WIP orchestrateur — finalisation (session 4) (2026-09-24)
- Plan : `docs/design/v2-finalisation.md`.
- **État final (24/09) : tous les lots fusionnés.** Prochaine étape éventuelle : portraits après le 1er octobre ; restes listés dans `docs/sessions/2026-09-23-session-04.md`.
- Clé OpenRouter bloquée jusqu'au 1er octobre (limite propre 100 $/mois) : pas de portraits cette session.

## Grappe `g`

### g1-rules.md — Lot G1 « Dernières règles inertes » — état (2026-09-24)
- Branche : `worktree-agent-aae49b73351b126ff`.
- 1. [x] `recruit_slots` : 2 + 1 (capitale) + effets ; `OrderError::RecruitQueueFull` ; l IA respecte les places libres.
- 2. [x] `Piety` : traits → `religion::effective_piety` (faveur, hérésie) ; bâtiments → +1 piété/an au souverain par 10 (plafond 3), hiver.

### g2-ai.md — Lot G2 « IA : alignement historique » — état (2026-09-24) [restes]
- Branche : `g2-ai-historical`. Périmètre : `core/crates/ai` (aucune modification de `sim-campaign` ni des
- données : l'ordre `SendGift` existant sert aux subsides). Réglages : `docs/design/m9-ai.md` § 5 ; tableau
- avant/après : `docs/status.md` (« Alignement historique G2 »). Tests : `core/crates/ai/tests/g2.rs` (4).

### g3-battle-techs.md — G3 — technologies dans la bataille 3D (limite connue G1) (2026-09-24) [restes]
- Lever la limite connue G1 : « la bataille 3D n'applique toujours pas les bonus des
- technologies (seuls ceux des bâtiments de la province de levée passent dans
- `core/crates/sim-campaign/src/battle_request.rs::side_setup` applique désormais

### g4-burgundy.md — Lot G4 « Bourgogne et Brabant » — état (2026-09-24)
- Branche : `worktree-agent-a734b058e294e9537` (à partir de `main` ac6b7c2). Périmètre : `core/crates/ai`
- (alignement), `data-model` (struct `AiAlignment`, chargée depuis `data/ai/alignment.json`), `sim-campaign`
- (constantes de raisons exposées : `GIFT_REASON`, `AT_WAR_REASON`, `PERJURY_REASON`, `AGGRESSION_REASON`),

### g5-neighbors.md — Lot G5 « Voisinage réel » — état (2026-09-24)
- Branche : `worktree-agent-a8e6ab2920478f32d` (à partir de `main` f52fd92).
- Objet : `CampaignState::are_neighbors` sur l'adjacence de la carte (`movement::land_neighbors`, graphe de
- `data/map/provinces.geojson`) au lieu des `neighbors` des fichiers de province (6/132 renseignés) ;

## Grappe `historien`

### historien.md — WIP — session historien (histoire et savoir) (2026-09-24)
- Conception : `docs/design/2026-09-23-histoire-et-savoir.md`.
- Coordination : sessions parallèles orchestrateur (2b, F1-F9), ui-tw (e3), visual (76).
- Ne stager que ses propres chemins. F8 (encyclopédie) doit s'appuyer sur le Codex.

### historien-suite.md — WIP — Historien, suite (pistes du README histoire) (2026-09-27)
- Session historien, 24 septembre 2026. Source : `docs/histoire/README.md` (« Pistes pour la suite ») et audit § 7.
- Hors lot : portraits des nouveaux personnages (clé OpenRouter après le 1er octobre).
- ~~IA : `prov_normandie_ouest` reçoit au premier tour une armée sans général~~ — résolu en P1 (`sim_campaign::frontier`, classification unique setup/IA ; plus aucune garnison scindée au premier tour). Godefroy d'Harcourt pourrait…

## Grappe `m`

### m1-navgrid.md — M1 — grille de navigation (état) (2026-09-24)
- Spec : `docs/design/2026-09-24-mouvement-libre.md` § 2. Branche : `worktree-agent-aded02b061ab29ed8` (non fusionnée).
- `tools/cent_ans_tools/geo/navgrid.py` : étape du pipeline, appelée à la fin de `geo/build.py` et seule par `uv run --project tools cent-ans geo navgrid` (`--lenient` écrit la grille malgré des colonies isolées).
- `data/map/navgrid.png` : 2048², 8 bits, 0,37 Mo ; `map.json` gagne `navgrid`.

### m3-tour-ia.md — M3 — tour séquentiel et IA sur la grille (état) (2026-09-25) [restes]
- Spec : `docs/design/2026-09-24-mouvement-libre.md` § 3.4, § 4, § 7. ADR : `docs/decisions/0010-free-army-movement.md`.
- Branche : `m3-turn-ai` (worktree `agent-a2eb8f7f4bd84c3dd`), depuis `main` 93d466c (M1 + M2).
- Squelette : `data/ai/grid.json` + schéma `ai_grid.schema.json` + `AiGrid` (data-model, `GameData::ai_grid`) + test pytest.
- Réf. : ADR 0010

### m4-pont-ui.md — M4 — pont et interface du mouvement libre (état) (2026-09-25)
- Spec : `docs/design/2026-09-24-mouvement-libre.md` § 6 et § 7. Suit M2 (`docs/wip/m2-core-movement.md`).
- Branche : `worktree-agent-a3602d6ab1a3f3f59` (partie de main `93d466c`, main `45f7ad5` fusionné).
- Cœur : `path_plan.rs` (`CampaignState::plan_path` → `PathPlan { points, turn_ends, cost, cost_this_turn }`, fonction pure), tests `tests/m4_path_plan.rs` (1 tour, 3 tours et arrêt identique à la vraie marche, cible inatteignable,…

### m5a-vision.md — M5a — vision par rayon (état) (2026-09-25)
- Spec : `docs/design/2026-09-24-mouvement-libre.md` § 5. Branche : `m5a-vision` (worktree `agent-aef7911654fc760d5`), partie de main `fa7efb3c`, main `b5a9c4d7` (M5b) fusionné.
- Cœur : `vision.rs` réécrit (`CampaignState::vision` → `Vision { mask: VisionMask, provinces, lenders }`, `visible_provinces`, `visible_armies`) ; `GameData::province_raster` ; `VisionRules.province_seen_percent` (+ schéma, `data/…
- Tests Rust (`c1_vision.rs` réécrit : rayon armée, rayon colonie, allié, armée cachée, temps ≈ 1 ms par faction en release comme en debug).
- Réf. : commits fa7efb3c, b5a9c4d7

### m5b-equilibrage-docs.md — M5b — équilibrage du mouvement libre et documentation (état) (2026-09-25)
- Spec : `docs/design/2026-09-24-mouvement-libre.md` § 7-8. Suivi : `docs/wip/mouvement-libre.md`.
- Branche : `worktree-agent-a3c4f4bdfe00c9af3`, depuis main fa7efb3c (M1-M4).
- Mesure de référence (`settlements_probe 50 1..8`, release) : identique au M3 de `m3-tour-ia.md`.
- Réf. : commits fa7efb3c, a4508f81

## Grappe `p`

### p1-petits-points.md — P1 — petits points cœur (2026-09-24)
- Branche dédiée (worktree), fusion par l'orchestrateur.
- 1. `no_quarter` en campagne : `resolve_pending_battle` (sim-campaign/src/battle_request.rs).
- Vainqueur « pas de quartier » → chef vaincu pris = tué (pas de captif, pas de rançon H6),

### p2-rapport-smoke.md — WIP — P2 : rapport de saison + batailles ; smoke Godot (2026-09-24)
- Tâche (lot P2) : voir `docs/wip/finalisation.md` § « Défauts relevés »
- (« Rapport de saison : n'inclut pas les batailles résolues par le dialogue
- après la fin du tour ») + vérification/robustesse du smoke Godot.

### p2a-court.md — P2a — cour, fiche personnage, arbre familial (fichier de reprise) (2026-09-28) [restes]
- Lot Phase 2 du chantier PO (polish), ADR 0097, bible DA § 12. Brief : `docs/wip/po.md` §
- « Phase 2 », plan `docs/superpowers/plans/2026-09-27-po-polish.md`.
- Fichiers du lot (ne pas en sortir) :
- Réf. : ADR 0097, 0098
- Restes : **Choix pris pour ce lot (mécanique, sans trancher la question) :** `CourtPanel` et / **À trancher plus tard** (orchestrateur / PO6b) : soit modifier `map_ui.gd` pour réclamer

### p2f-fonts.md — P2f — `add_theme_font_size_override` restants → `UiType` (2026-09-28)
- Lot phase 2 de PO (polish). Voir `docs/wip/po.md` (case P2f) et la bible DA § 12.1/§12.2
- (échelle `UiType` : `TITLE`=26, `HEADING`=20, `BODY`=17, `CAPTION`=14).
- Remplacer les `add_theme_font_size_override(..., <nombre en dur>)` restants par

### p2g-layout.md — P2g — fenêtres centrales et `SaveLoadDialog` dans `UiLayout` (2026-09-28) [restes]
- Chantier PO phase 2 (`docs/wip/po.md`, `docs/wip/restes.md`). Branche `feat/p2g-layout`.
- Les fenêtres Techniques, Diplomatie, Cour, Fiche de personnage et `SaveLoadDialog` (depuis
- `start_menu` et `map_ui`) étaient restées hors de `UiLayout` : les reparenter dans la zone `MODAL`

## Grappe `s`

### s1-effondrement-murailles.md — WIP — S1 effondrement physique des murailles (2026-09-24) [restes]
- Spéc : consigne du lot S1 (visuel seulement), contexte `docs/design/m8-sieges.md` ; ADR 0007.
- Données `data/fx/siege_fx.json` + schéma `data/schemas/siege_fx.schema.json` + pytest
- `tools/tests/test_siege_fx_schema.py`.
- Réf. : ADR 0007

### s1-personnages.md — WIP — S1 personnages manquants (audit § 7) (2026-09-24) [restes]
- **État : terminé.** Source : `docs/histoire/audit-2026-09-23.md` § 7 « Personnages de 1337
- manquants importants ». Déjà présents (non retouchés) : Gautier de Mauny, William Montagu,
- Non créés en personnage (aucune faction adaptée : pas de Mérinides ni de Danemark) — fiche

### s2-noms-etain.md — S2 — Prénoms arabes andalous & ressource étain (2026-09-24)
- `data/names/names_ar.json` créé (arabe andalou, `cul_andalusi`, 30 prénoms masculins, 20 féminins, épithètes nasrides).
- `cul_andalusi` retiré de `data/names/names_es.json`.
- Test Rust ciblé (`core/crates/sim-campaign/src/dynasty.rs`, module `culture_names_tests`) : Grenade tire ses prénoms de `names_ar`, jamais de `names_es` ; `chr_yusuf_i.house == "Nasrides"` (pas castillan).

### s3-icones-regimes.md — WIP S3 — Icônes des régimes alimentaires et `res_tin` (2026-09-24) [restes]
- Branche : `worktree-agent-a4eb500d6919a0a8a`. Plan : `docs/wip/historien-suite.md` (lot S3).
- 7 icônes de régime (`data/diets/diet_*.json`) ajoutées à `tools/cent_ans_tools/icons_catalog.py`
- (catégorie `resource`, comme lue par `table_section.gd`) :

### s1-s2-physique-incendies.md — WIP — S1 effondrement des murailles + S2 incendies de siège (2026-09-28)
- Demande du joueur (24/09) : physique de destruction des murailles et incendies.
- **S1 (effondrement physique, rendu seulement)** : **fusionné dans main** (`41bb147`). Jolt activé, ADR 0007,
- `game/scripts/battle/wall_collapse_fx.gd`, réglages `data/fx/siege_fx.json`. Détails : `docs/wip/s1-effondrement-murailles.md`.
- Réf. : ADR 0007, 0008, 0099

## Grappe `session`

### session-05.md — Session 5 — crédit OpenRouter (19 €) et écarts restants (2026-09-24)
- Démarrée le 2026-09-24, interrompue le même jour par l'utilisateur (reprise plus tard).
- Nouvelle clé OpenRouter (≈ 19,6 $ de crédit au départ, `total_usage` du compte = 80,3766 $ sur
- 100 $ au démarrage) ; l'utilisateur autorise à tout consommer (plafond projet 50 $ inchangé,

## Grappe `ui`

### ui-tw.md — WIP — interface « à la Total War » (session e3, parallèle à l'orchestrateur de finalisation) (2026-09-24)
- Plan : `docs/design/2026-09-23-audit-ui-total-war.md`.
- Coordination : orchestrateur = session game-project-2b ; refonte visuelle = game-project-76 (possède shaders, map rendering, battle_meshes/terrain).
- Prochaine étape : rien de bloquant côté ui-tw. Interface de bataille compacte = F5 (orchestrateur). Bannières 3D = session visual (fait sur la carte). Signalé à 2b : Philippe VI meurt avant 1340 dans la partie de capture « chroni…

### ui1-enluminures.md — WIP — UI1 : habillage « manuscrit enluminé » de l'interface (2026-09-25)
- Demande (2026-09-25) : l'UI n'est qu'une bande sépia et des boutons plats ; la rendre plus
- belle, avec des assets « moines », enluminures. Décision : ADR 0050.
- Générateur `tools/cent_ans_tools/ui_illumination.py` + commande `cent-ans assets ui-illumination`
- Réf. : ADR 0050

### ui3-interface.md — UI3 — interface de campagne, suite (lots U5, U7 fin, U10, U11, U12, U8 de l'audit A3) (2026-09-25)
- Branche : `worktree-agent-ae5685646cc8b59ac` (main + UI2 `worktree-agent-a817c3f698be6306c` fusionnés).
- Source : `docs/audit/a3-ui.md` § 3 et § 8 ; suite de `docs/wip/ui2-interface.md`.
- Captures : `docs/audit/captures/ui3/` (avant/après par lot). Propriétaire exclusif de

## Grappe `visuel`

### visuel.md — WIP — chantier visuel (session 5, orchestrateur game-project-76) (2026-09-24)
- Plan : `docs/design/visuel-semi-realiste.md`. Branche `visual` (worktree `.claude/worktrees/visual`).
- Captures « avant » : scratchpad de session (à copier dans docs/img/visuel/).
- 19 h 45 : V1 carte commité (d61132f). Agents V2, V4 puis V3 lancés dans leurs worktrees depuis `visual`.

## Grappe `ar`

### ar1-illustrations.md — WIP — AR1 : habillage illustré (chargements, vignettes d'événements, fins) (2026-09-25) [restes]
- Branche : `worktree-agent-aaa4cf9bc2224eea2`.
- Données : `data/ui/illustrations.json` (schéma `data/schemas/illustrations.schema.json`) :
- `loading.screens` (contexte battle/siege/naval/campaign, titre, citation de chroniqueur),

## Grappe `au`

### au1-audio.md — AU1 — audio (banque libre T4, bataille spatialisée T3, ambiances de carte, mixage) (2026-09-25)
- Agent AU1, vague 5 (session de nuit 7). Sources : `docs/audit/a5-technique.md` (§ 3, lots T3/T4),
- `docs/audit/a4-assets-libres.md` (§ 6), `docs/wip/b3-musique-camera.md`, `docs/wip/d0-assets.md`.
- Squelette : bus (`AudioBuses`), banque (`SoundBank` + `data/audio/sound_bank.json` + schéma),

## Grappe `br`

### br1-batiments.md — BR1 — Bâtiments réalistes (campagne et bataille) (2026-09-25) [restes]
- Branche `br1-buildings`, worktree `../gp-br1`. ADR 0021. Session séparée (le joueur est absent
- plusieurs heures, autonomie complète).
- 0. Squelette : ADR, wip, textures Poly Haven (`game/assets/textures/buildings/`)
- Réf. : ADR 0021, 0047 ; commits 8a2198c9

### br3b-equilibre-paris.md — BR3b — équilibre du siège de Paris après BR3 (2026-09-25) [restes]
- Addendum « BR3b » à l'ADR 0047. Branche d'agent `worktree-agent-a212c40bf9fdb8559` (depuis main 21f2b125).
- Paris 8-12/20 (victoires de l'assaillant, sonde `br3_assault_probe`), générique 15-19/20, Rouen ≤ 3/20,
- autres villes emblématiques ±3/20 par rapport à la référence, maisons brûlées à Paris ≤ 25 en moyenne ;
- Réf. : ADR 0047 ; commits 21f2b125, d29e5684, e885d011

## Grappe `bv`

### bv1-bataille-vivante.md — Lot BV1 — Bataille vivante (1) : volées, sang au sol, poussière, taille des unités (2026-09-25)
- Branche `worktree-agent-a3fb69eecbaf4f8a1` (main fusionné en dernier à `821a4ceb` : V2, AU1, V3 et L1 compris). Backlog :
- `docs/audit/backlog-tw.md` § Bataille (idées J). Coordination : V2 (soldats VAT) possède le maillage
- et le shader des soldats, V3 l'environnement global et AU1 l'audio (bus, pool, banque). BV1 ne
- Réf. : ADR 0016 ; commits 821a4ceb, abc03449

### bv2-bataille-vivante.md — Lot BV2 — Bataille vivante (2) : morts, chutes, chocs de cavalerie, sang, démembrements (2026-09-25)
- Branche `worktree-agent-af32527b47b35b47a` (a fusionné `main` à `fa7efb3c`, avec V2, AU1 et V3).
- Backlog : `docs/audit/backlog-tw.md` § Bataille (idées J). ADR :
- `docs/decisions/0022-chocs-morts-et-sang-rendus.md`.
- Réf. : ADR 0022 ; commits fa7efb3c

## Grappe `cm`

### cm2-carte-parchemin-meteo.md — CM2 — carte de campagne : vue stratégique parchemin, météo, lumière de fin de tour (2026-09-25)
- Branche `worktree-agent-a7241565332549af6` (partie de `main` `ad08ea16`).
- 1. Vue stratégique parchemin (fondu selon la distance caméra, zoom maximal).
- 2. Météo de campagne : `core/` (déterministe : graine, date, province), pont, rendu (pluie, neige,
- Réf. : ADR 0027 ; commits ad08ea16, 44bcdc16

## Grappe `cv`

### cv1-campagne-vivante.md — CV1 — Campagne vivante (partie 1) (2026-09-25)
- Branche : `worktree-agent-ae5263563bc920e75` (partie de `main` `93d466c`). Rendu seulement :
- aucune règle, l'information vient du pont (`get_date_label`, `get_turn`, `get_province_state`,
- `game/scripts/map/campaign_life.gd` (`CampaignLife`) : nœud unique branché dans `campaign_map.gd`

### cv2-armees-carte.md — Lot CV2 — Armées et flottes figurées sur la carte de campagne (2026-09-25)
- Branche `worktree-agent-a03415e18588b6fde` (main fusionné le 2026-09-25 : L1, M3, R1… sans
- conflit ; dylib reconstruite, import et smoke verts). Rendu seulement : aucune règle,
- `movement.rs` intact ; lecture de `get_army` et de l'animation M4 (`ArmyMarkers.place_marker`).

### cv3-0-carte.md — CV3-0 — Défauts de la carte de campagne (2026-09-27)
- Spec : annexe A de `docs/design/2026-09-27-campagne-vivante.md` (section « Défauts de la carte
- de campagne relevés sur les captures »).
- 1. Caméra trop proche au max — fait : `campaign_camera.gd` max_distance 1500->2600,

### cv3-1-postures.md — CV3-1 — Postures d'armée et résultats nuancés (core) (2026-09-27) [restes]
- Branche : `feat/cv3-1-postures`. Spec : `docs/design/2026-09-27-campagne-vivante.md` § 1, § 3.
- Orchestration : `docs/wip/cv3-campagne-vivante.md`. ADR : `docs/decisions/0094-postures-embuscade.md` (0093 pris).
- `core/crates/sim-battle/src/setup.rs` :
- Réf. : ADR 0094 ; commits 29297f7d

### cv3-2-embuscade.md — CV3-2 — Ouverture d'embuscade, marche forcée, camp retranché (bataille 3D) (2026-09-27)
- Branche : `feat/cv3-2-ambush-battle` (base `main` 9bdd69f9, contient CV3-1 5096da0b).
- Spec : `docs/design/2026-09-27-campagne-vivante.md` § 1.2-1.3 ; ADR 0094 (§ « Suite CV3-2 ») ;
- contrat CV3-1 de `core/crates/sim-battle/src/setup.rs` inchangé ; format de rejeu inchangé.
- Réf. : ADR 0094 ; commits 9bdd69f9, 5096da0b

### cv3-3-rencontres.md — CV3-3 — Rencontres sur la carte de campagne (2026-09-27)
- Branche : `feat/cv3-3-encounters`. Spec : `docs/design/2026-09-27-campagne-vivante.md` § 2.
- Données : `data-model/src/entities/encounter.rs` (`Encounter`, `EncounterSpawn`, `EncounterOption`,
- `EncounterOutcome` battle/join, `EncounterRules`), id `enc_`, chargement `data/encounters/` et

### cv3-4-ui.md — CV3-4 — Interface de campagne : postures, rencontres, classes de résultat (2026-09-27)
- Branche : `feat/cv3-4-campaign-ui`. Spec : `docs/design/2026-09-27-campagne-vivante.md` § 4 (UI).
- Pont utilisé : `get_stance_options`, `get_last_battle_outcome`, `get_encounter_sites`,
- `get_pending_encounters`, `choose_encounter_option` (lots CV3-1, CV3-3).

### cv3-5-zone-lord.md — CV3-5 — Zone atteignable à deux tons et lord « à l'échelle TW » (2026-09-27) [restes]
- Branche : `feat/cv3-5-zone-lord`. Spec : `docs/design/2026-09-27-campagne-vivante.md` § 4.
- Orchestration : `docs/wip/cv3-campagne-vivante.md`.
- Core : `CampaignState::reachable_cells(data, army) -> ReachableCells { this_turn: Vec<(Cell, u32)>,

### cv3-6-ia-equilibrage.md — CV3-6 — IA des postures et des rencontres, équilibrage CV3 (2026-09-27)
- Branche : `worktree-agent-abcec86700c5ee43d` (worktree privé, base 5704fa8c).
- Spec : `docs/design/2026-09-27-campagne-vivante.md` § 0 et § 5. ADR 0094 (addendum CV3-6), ADR 0085 (bande EQ6).
- L'IA de campagne réelle est `core/crates/ai/src/campaign.rs` (`plan_armies`) + `grid.rs` ;
- Réf. : ADR 0085, 0094 ; commits 5704fa8c

### cv3-campagne-vivante.md — CV3 — Campagne vivante (orchestration) (2026-09-27) [restes]
- Spec : `docs/design/2026-09-27-campagne-vivante.md`. Plan détaillé : voir la section « Lots » ci-dessous.
- ADR réservée : `docs/decisions/0094-postures-embuscade.md`. Captures : `docs/img/cv3/`.
- Couvert d'une case : `CoverMap` (data-model) depuis `forest_kind.png` + `wetlands.json`, repli `Terrain` de province.
- Réf. : ADR 0094 ; commits c4c86f66, 5096da0b, 348fe5a7

## Grappe `da`

### da2-portraits-vivants.md — DA2 — Portraits vivants (état de travail) (2026-09-25)
- Branche : `worktree-agent-ade9b18af852c5bfe` (worktree `.claude/worktrees/agent-ade9b18af852c5bfe`,
- contient `feat/da-direction-artistique`). La fusion est faite par l'orchestrateur DA, pas par
- l'agent. ADR : `docs/decisions/0063-portraits-vivants.md` (0057-0060 laissés libres : trous vus
- Réf. : ADR 0063 ; commits 3c99fe02

### da-direction-artistique.md — DA — Direction artistique (orchestration) (2026-09-26)
- Demande du joueur (25/09 soir) : « tu es le directeur artistique, que manque-t-il au jeu
- comparé à Total War et Crusader Kings ? » puis « ok pour la recommandation ».
- Bible : `docs/design/2026-09-25-bible-da.md` (à lire avant tout lot DA).
- Réf. : ADR 0060, 0063, 0064, 0065, 0066, 0067 ; commits 6fc9d9fc, 495470d9, fc968b5f

### da1-heraldique.md — DA1 — Armoiries des maisons et héraldique des figurines (2026-09-26)
- Branche : `worktree-agent-addd87848de848c53` (worktree `.claude/worktrees/agent-addd87848de848c53`),
- basée sur main + merge de `feat/da-direction-artistique` (bible DA). Bible :
- `docs/design/2026-09-25-bible-da.md`. ADR : `docs/decisions/0064-armoiries-des-maisons.md`.
- Réf. : ADR 0064

### da1b-meubles-etendards.md — DA1b — Meubles héraldiques dessinés et étendards aux armes du général (2026-09-26)
- Branche : `feat/da1b-meubles-etendards` (worktree `.claude/worktrees/agent-ad8baf6e847fddbc3`),
- basée sur main (4c627f4a, DA1 fusionné). ADR : section « Révision DA1b » de
- `docs/decisions/0064-armoiries-des-maisons.md`.
- Réf. : ADR 0064 ; commits 4c627f4a

### da3-marqueurs-carte.md — DA3 — Marqueurs de carte (langage unique) (2026-09-26)
- Lot DA3 de `docs/wip/da-direction-artistique.md`, bible `docs/design/2026-09-25-bible-da.md` § 7-8.
- Branche `worktree-agent-a4046e3d4756f0516` (fusionnée avec `feat/da-direction-artistique` et main).
- Plafond du lot : 1,5 $ ; **dépensé 0,59 $** (sonde 3 images 0,14 $ + 10 images 0,45 $). ADR 0066.
- Réf. : ADR 0066 ; commits f6ab5a23

### da4-musique.md — DA4 — musique d'époque et bataille en couches (2026-09-26)
- Lot DA4 (agent solo). Bible : `docs/design/2026-09-25-bible-da.md` § 9. ADR : `docs/decisions/0060-musique-d-epoque.md`.
- Recherche de sources libres (Wikimedia Commons, licence relue page par page via l'API
- `extmetadata.LicenseShortName` ; Freesound CC0 relu page par page comme AU1) : 11 nouvelles
- Réf. : ADR 0060

### da5-boutons-icones.md — DA5 — Boutons-médaillons et icônes d'action à l'encre (2026-09-26)
- Branche : `worktree-agent-aa84e124b506691c1` (a fusionné `feat/da-direction-artistique`).
- Bible : `docs/design/2026-09-25-bible-da.md` § 5, § 8. ADR 0065. Plafond du lot : 5 $.
- Catalogue `data/ui/icons_ink.json` + schéma `data/schemas/icons_ink.schema.json` : 78 icônes
- Réf. : ADR 0065

### da5b-icones-entites.md — DA5b — Icônes d'entité en miniatures peintes (2026-09-26)
- Branche : `worktree-agent-a19e7d997471a9770` (base main 9bfbf27f, DA5 fusionné).
- Bible `docs/design/2026-09-25-bible-da.md` § 8 ; ADR 0065, section « DA5b ». Plafond 6 $.
- Inventaire : 153 identifiants encore en SVG ; 101 ont une illustration peinte
- Réf. : ADR 0065 ; commits 9bfbf27f

### da6-vegetation-bataille.md — Lot DA6 — Végétation de bataille (direction artistique) (2026-09-26)
- Branche `feat/da6-vegetation-bataille` (worktree `.claude/worktrees/agent-abf402aacfc74bffd`).
- Bible : `docs/design/2026-09-25-bible-da.md` § 3.3, § 6 « Végétation », § 10 ligne 6.
- Suivi DA : `docs/wip/da-direction-artistique.md`. ADR 0067. Rendu seulement (aucun Rust).
- Réf. : ADR 0067 ; commits 4c627f4a

### da7a-ars-nova.md — DA7a — vrais enregistrements libres d'Ars nova (2026-09-26)
- Branche : `worktree-agent-a0b3b1e6efa501ff2`. Lot données seulement (pas de build Rust).
- Remplacer en `primary` les rendus MIDI d'Ars nova (France, Italie) par de vrais enregistrements
- interprétés sous licence libre (PD, CC0, CC BY, CC BY-SA ; jamais NC/ND). MIDI conservé en repli.
- Réf. : ADR 0060

### da7b-saturation.md — Lot DA7b — Saturation en automne et en bocage (2026-09-26)
- Branche `feat/da7b-saturation` (worktree `.claude/worktrees/agent-ae97dd783f8ba4906`).
- Bible : `docs/design/2026-09-25-bible-da.md` § 3.3 (saturation HSV moyenne ≤ 35 % en plein jour).
- Suite de DA6 (ADR 0067 § « DA7b », `docs/wip/da6-vegetation-bataille.md`). Rendu et données
- Réf. : ADR 0067

### da7c-icones-traits.md — DA7c — une icône pour chaque trait de personnage (2026-09-26)
- Bible `docs/design/2026-09-25-bible-da.md` § 8, ADR `docs/decisions/0065-boutons-et-icones-enlumines.md`
- (§ DA7c). Plafond de dépense **4 $** (distinct des 5 $ de DA5, déjà consommés à 4,84 $).
- 1. Inventaire : les 59 traits (`data/traits/*.json`) n'avaient pas d'icône propre ; les
- Réf. : ADR 0065

### da7d-chevauchements.md — DA7d — Chevauchements des marqueurs de ville (régions denses) (2026-09-26)
- Lot DA7d de `docs/wip/da-direction-artistique.md` (vague DA7). ADR 0066, section « DA7d ».
- Branche `feat/da7d-marker-overlap` (worktree agent). 0 $.
- **Un seul placement** : `SettlementLayer.declutter()` de DA7d (marqueurs + noms, par priorité,
- Réf. : ADR 0066

## Grappe `difficulte`

### difficulte.md — Lot DF1 — niveaux de difficulté de campagne (2026-09-25) [restes]
- Branche : `feature/df1-difficulty` (worktree agent). Ne pas merger soi-même.
- Données `data/rules/difficulty.json` + schéma `data/schemas/difficulty_rules.schema.json` + test pytest.
- `data-model` : `DifficultyRules` (défaut = miroir du fichier), chargé dans `GameData::difficulty`.
- Réf. : ADR 0037 ; commits 580fc208

## Grappe `dp`

### dp1-diplomatie.md — DP1 — Diplomatie à la Total War (négociation, buts de guerre, écran plein) (2026-09-25) [restes]
- Branche : `worktree-agent-a4e14689f1209a2c9`. ADR : `docs/decisions/0025-negociation-et-buts-de-guerre.md`.
- Cœur : `core/crates/sim-campaign/src/negotiation.rs` (articles, évaluation, contre-proposition,
- buts de guerre, fatigue, paix de l'IA). Branchements : `diplomacy.rs` (`Proposal::Treaty`,
- Réf. : ADR 0025
- Restes : Graines 4 (48 %) et 3 (52 %) du siècle sous la cible : cause non analysée (`dp1_probe` affiche / L'accès militaire ne joue que sur le ravitaillement (pas de règle d'intrusion en paix). / Accord commercial autonome : à relier à C5 (`integration/tw`) quand il sera sur main.

## Grappe `ep`

### ep3-eau-chemins.md — Lot EP3 — Eau et chemins (chantier « batailles épiques ») (2026-09-25) [restes]
- Branche `ep3-eau-chemins` (worktree `.claude/worktrees/agent-a74358ec7ea9828be`). Suivi du chantier :
- `docs/wip/epic.md`. ADR : `docs/decisions/0033-eau-ponts-routes.md`.
- Rivières variées (largeur 10-40 m, affluent, ruisseaux, berges escarpées ou marécageuses, bras
- Réf. : ADR 0033 ; commits 1f38c4ef

### ep4-son-melee.md — EP4 — son de mêlée de proximité (2026-09-25) [restes]
- Lot du chantier « batailles épiques » (`docs/wip/epic.md`). Objectif : bataille qui sonne
- massive — cris et fracas d'armes en zoomant près d'un combat, émetteurs par front de mêlée,
- couches selon la distance caméra. S'appuie sur AU1 (`docs/wip/au1-audio.md`,

### ep5-porte-etendards.md — Lot EP5 — Porte-étendards (chantier « batailles épiques ») (2026-09-25)
- Branche `worktree-agent-ae217ad35aaf7a266`. ADR : `docs/decisions/0034-porte-etendards.md`.
- Plan d'ensemble : `docs/wip/epic.md`.
- Figurines du manifeste `game/assets/models/battle_skinned/manifest.json` (nom `kind_variant`) :
- Réf. : ADR 0034 ; commits 64bccfdd, 81b87442

### ep6-villages-decor.md — Lot EP6 — Villages et décor du champ de bataille (chantier « batailles épiques ») (2026-09-25)
- Branche `ep6-villages-decor` (worktree `.claude/worktrees/agent-afdabf24728dfdac8`). Suivi du
- chantier : `docs/wip/epic.md`. Dépend d'EP1 (taille du champ) et d'EP3 (eau, ponts, routes).
- Le champ de bataille doit ressembler à une campagne du XIVe siècle : hameaux en rue ou groupés
- Réf. : ADR 0052, 0061 ; commits d0be0f2b, 25bf6fbf

### ep8-mise-en-scene.md — Lot EP8 — Mise en scène des batailles (2026-09-25)
- Branche `worktree-agent-a8a54631be0d814c5`. Plan du chantier : `docs/wip/epic.md`. ADR : 0055
- (à écrire ; 0053 est pris ailleurs, vérifier avant de commiter).
- **Heure du jour (cœur)** : `data/rules/battle_time_of_day.json` (schéma
- Réf. : ADR 0055 ; commits 034351af

### ep9-batailles-decisives.md — EP9 — Batailles décisives (2026-09-25)
- Lot : les batailles de campagne doivent se décider d'elles-mêmes (recette Q3, point 12).
- Branche : `worktree-agent-a6207325088470391`. ADR 0056.
- Règles `data/rules/battle_decision.json` (+ schéma, pytest) ; `sim-battle/src/decision.rs`,
- Réf. : ADR 0056

### ep1-echelle-massive.md — Lot EP1 — Échelle massive (batailles épiques) (2026-09-26)
- Branche `worktree-agent-a8c20286fc6084f08`. Plan du chantier : `docs/wip/epic.md`. ADR : 0076.
- Batailles de 15 000 soldats et plus, fluides (≥ 40 i/s à 15 000 en Haut, ≥ 30 à 25 000).
- Paliers par effectif total (soldats simulés des deux camps, réserves comprises) :
- Réf. : ADR 0076

### ep10-deroute-contagion.md — EP10 — direction de la déroute et contagion de moral (2026-09-26)
- Branche : `worktree-agent-adc8c89fbb2be1ac1` (worktree agent), main fusionné (SG5, 222080a4).
- ADR : `docs/decisions/0068-direction-de-la-deroute-et-contagion.md` (mesures avant/après).
- `data/rules/battle_rout.json` + schéma + pytest ; `sim-battle/src/rout.rs` ; branché dans `sim.rs`
- Réf. : ADR 0068 ; commits 222080a4

### ep11-poussee-lignes.md — EP11 — Poussée continue des lignes en mêlée (2026-09-26)
- Branche : worktree agent-a5b7ba925e656a5c7. ADR : `docs/decisions/0071-poussee-continue-des-lignes.md`.
- Règles `data/rules/battle_push.json` + schéma `battle_push_rules.schema.json` + pytest
- `tools/tests/test_battle_push_schema.py`.
- Réf. : ADR 0071

### ep12-blesses-armes.md — Lot EP12 — Blessés qui rampent, fuyards qui jettent leurs armes (2026-09-26)
- Branche `worktree-agent-ab10bccc115923724`. Rendu seulement (game/, tools/blender_scripts, data/fx) ;
- aucun changement du cœur. ADR 0070 (renumérotée 0073 → 0070 à la fusion : CT1 a repris 0073 ; 0071 laissée à EP11).
- 1. Kit Blender (`battle_skinned_poses.py`) : clips `crawl` (7 s), `wounded_sit` (6 s),
- Réf. : ADR 0070

### ep13-rejeu.md — EP13 — Rejeu d'après bataille (2026-09-26) [restes]
- Branche `feat/ep13-replay` (worktree agent). ADR 0072 (numéro choisi pour laisser 0070-0071 à EP11/EP12).
- Simulation déterministe : on enregistre le départ (`ReplayStart` : setup, graine, échelle, site
- historique) et les entrées horodatées par pas (`ReplayEntry`), puis on re-simule.
- Réf. : ADR 0072 ; commits b04b88ce

### ep7-cartes-historiques.md — Lot EP7 — Cartes historiques : Crécy, Poitiers, Azincourt (2026-09-26) [restes]
- Branche `feat/ep7-historical-maps` (worktree `.claude/worktrees/agent-aa22e819038559fe3`). Suivi du
- chantier : `docs/wip/epic.md`. ADR : `docs/decisions/0035-cartes-historiques.md`.
- Dépend d'EP1 (taille du champ), EP2 (horizon), EP3 (eau), EP6 (`DecorPlan`), EP8 (`set_start_hour`),
- Réf. : ADR 0035 ; commits 9bfbf27f

### ep8b-horloge.md — EP8b — Correctif du bandeau d'heure de bataille (2026-09-26)
- Branche : worktree courant. ADR : 0055 (addendum « EP8b — correctif du bandeau »).
- Bataille historique d'Azincourt (10 h 30) : le bandeau affiche « Midi » vers 2 min 50 de bataille
- jouée. Semble trop rapide.
- Réf. : ADR 0055

### ep9b-duel-attaquant.md — EP9b — Duel de l'attaquant et milice en second échelon (2026-09-26)
- Lot : correction d'EP9 mesurée par SG4. ADR 0056 § EP9b. Branche `ep9b-duel-attaquant`
- (worktree `agent-a7bf63ca963f71d60`), partie d'`integration/night` (main + SG4), main fusionné
- Reproduction : `sim-battle/tests/ep9b_duel.rs` (60 régiments/camp, plat, sans pieux,
- Réf. : ADR 0046, 0056

## Grappe `eq`

### eq1-equilibre.md — EQ1 — équilibre (trouble, Angleterre, banqueroutes, couleuvriniers) (2026-09-25)
- Branche : `worktree-agent-a6c791c0cae04ee1a` (C4 + C5 + DP1 + main fusionnés). ADR :
- `docs/decisions/0029-revoltes-et-couts-d-evenements.md`.
- Même machine, build release. « Avant » = C5R + DP1 + main (74ffeb1a), « après » = EQ1.
- Réf. : ADR 0029 ; commits cb553f64, 74ffeb1a

### eq2-equilibre.md — EQ2 — équilibre (impôt Haut, troubles bloqués, paliers de départ, Aide féodale, guerre FR-EN) (2026-09-25)
- Branche : `worktree-agent-a781586b50d0b6c1b` (EQ1 + main fusionnés).
- Tests : 617 réussis, 0 échec ; clippy propre.
- Référence : balance_probe 8 × 200 et century_probe 5 × 464 (sur EQ1 + main).
- Réf. : ADR 0037, 0049 ; commits 11ad6e96

### eq4-equilibre-combine.md — EQ4 — sonde d'équilibre combinée (EQ1 + EQ2 + EQ3 + DP2 + DF1 + C4/C5 + SG4) (2026-09-26)
- Branche : `worktree-agent-a2b33defc792e3eab` (main e384848c fusionné).
- `century_probe` étendu (tableau « EQ4 — tableau combiné ») : révoltes, sièges engagés et
- leur issue, prises directes, boule de neige (1re faction en part des provinces contrôlées)
- Réf. : ADR 0054 ; commits e384848c

### eq5-ia-banqueroutes-intrusions.md — EQ5 — IA de campagne : banqueroutes chroniques et intrusions (2026-09-26)
- Branche : `worktree-agent-a745acdb29a9168ed` (main + EQ4 fusionnés). ADR 0069 (renumérotée : 0068 pris par EP10).
- Sonde `century_probe` : `ECON_TRACE=fac_x` (budget d'une faction par tour, événements,
- écart inexpliqué du trésor), `TRESPASS_TRACE=1` (armées en intrusion en fin de tour),
- Réf. : ADR 0069

### eq7-cavalerie-attend.md — EQ7 — la cavalerie attend son infanterie sous les flèches (2026-09-26)
- Suite de l'ADR 0052. ADR **0083**. Branche `feat/eq7-cavalry-waits`, worktree `../gp-eq7`.
- sonde `tests/eq7_cavalry.rs` (`probe_mixed_battle`, `probe_symmetric_battle`)
- règle `waits_for_foot` dans `plan_horse` (étapes 1b et 5, assaillant seulement),
- Réf. : ADR 0052, 0083 ; commits d898672c

## Grappe `l`

### l1-paris.md — L1 — Villes emblématiques : Paris d'abord (2026-09-25)
- Branche `worktree-agent-a36a911aec4fbbb5c` (depuis `main` 93d466c). Backlog : `docs/audit/backlog-tw.md`
- § « Villes emblématiques ». ADR : `docs/decisions/0015-landmark-cities.md`. Captures : `docs/audit/captures/l1/`.
- Gabarit : plan `data/landmarks/<id>.json` (schéma `data/schemas/landmark.schema.json`), coordonnées
- Réf. : ADR 0015

### l2-villes.md — L2 — Villes emblématiques (suite) : Londres, Avignon, Calais, Rouen, Bordeaux, Bruges (2026-09-25)
- Gabarit L1 (`docs/wip/l1-paris.md`, ADR 0015 + addendum L2) appliqué à six villes. Captures :
- `docs/audit/captures/l2/`.
- Schéma étendu (`data/schemas/landmark.schema.json`) : `monuments[].params` (gabarits paramétrés,
- Réf. : ADR 0015

### l3-materiaux-sieges.md — L3 — Villes emblématiques : matériaux et sièges (2026-09-25)
- Branche `worktree-agent-af2974d71a33b3dc3`. Suite de L1 (`docs/wip/l1-paris.md`) et L2
- (`docs/wip/l2-villes.md`), ADR 0015 et **ADR 0026** (sièges dans le plan). Captures :
- `docs/audit/captures/l3/`.
- Réf. : ADR 0015, 0026

## Grappe `mm`

### mm1-menu.md — MM1 — première impression : écran titre, menu, choix de faction, chargement, introduction (2026-09-25) [restes]
- Agent MM1 (session 7). Sources : `docs/audit/a3-ui.md` (§ 3.1, M1-M6), `docs/wip/au1-audio.md`,
- `docs/wip/v3-atmosphere.md`, ADR 0014 (figurines skinnées) et 0015 (Paris).
- Données : `data/ui/front_end.json` + schéma `data/schemas/front_end.schema.json` + test
- Réf. : ADR 0014

## Grappe `mouvement`

### mouvement-libre.md — Orchestration : mouvement libre des armées (2026-09-25) [restes]
- Spec : `docs/design/2026-09-24-mouvement-libre.md`. ADR : `docs/decisions/0010-free-army-movement.md`. Conception validée par le joueur le 2026-09-24.
- 2026-09-24 : le joueur valide la spec telle quelle (« fais le »).
- 2026-09-24 : la session 6 (TW) est prévenue que son lot C2 « zone de contrôle » (vague 6) recoupe M2 ; proposition de le suspendre jusqu'à la fusion de M2. C2 est suspendu.
- Réf. : ADR 0010 ; commits e8056c2d, 99db2366

## Grappe `nv`

### nv1-naval.md — Lot NV1 — Batailles navales (à la Total War) (2026-09-25) [restes]
- Branche `worktree-agent-a5b40232e9b8caf20`. ADR : `docs/decisions/0028-batailles-navales.md`.
- Données : `data/naval/ships/ship_{cog,nef,galley,barge}.json`, `data/naval/rules.json`,
- `data/naval/fleets.json` (réserves, mers, fusiliers marins, noms des mers),
- Réf. : ADR 0028

### nv2-naval.md — Lot NV2 — Finitions navales (2026-09-25) [restes]
- Branche `worktree-agent-a8e48e736810f5f6e`. Suite de NV1 (ADR 0028, complément NV2 ;
- `docs/wip/nv1-naval.md`). Captures : `docs/audit/captures/nv2/`.
- Avant : 1 abordage à la fois, 1-2 navires anglais à l'abordage, Français rendus sous les flèches.
- Réf. : ADR 0028

## Grappe `pf`

### pf1-perfs.md — PF1 — performances (préréglages, zoom comté, fuite particules, user://) (2026-09-25)
- Agent PF1, 25/09. Machine partagée : **charge moyenne 20 à 230** pendant les mesures (autres
- agents : builds Rust, Godot). Les comparaisons fiables sont celles faites dans le même processus
- (`--map-ab`, niveaux alternés sur 4 tours) et les compteurs (primitives, appels de dessin).
- Réf. : ADR 0031, 0036 ; commits cb1b4416

## Grappe `q`

### q3-recette.md — WIP Q3 — recette « comme un joueur » après ~10 sessions fusionnées (2026-09-25) [restes]
- Branche `worktree-agent-aaf83c5a43e966bec`, base main 9fe0550a. Rapport : `docs/audit/q3-recette.md`,
- captures `docs/audit/captures/q3/` (JPEG 1280 px, script de copie dans le scratchpad).
- Pilote : `game/tests/q3_playtest.gd` (dérivé de Q1 ; phases à la carte, voix mesurées sur le bus
- Réf. : commits 9fe0550a, 544aec3c, 85e8efd9

### q4-corrections.md — Q4 — corrections après la recette Q3 (2026-09-25)
- Branche : `worktree-agent-a8324042adc365e4d` (Q3 + main fusionnés). Source : `docs/audit/q3-recette.md`.
- Captures : `docs/audit/captures/q4/`. Pilote : `game/tests/q3_playtest.gd` (`user://settings.cfg`
- sauvegardé puis restauré).
- Réf. : commits 43be1ab3, 7d160027, 0323ae7b

### q5-recette.md — WIP Q5 — recette « comme un joueur » (2026-09-26) (2026-09-26) [restes]
- Demande du joueur : tester le jeu comme un joueur, identifier les problèmes, puis les corriger
- en autonomie. Pilote `game/tests/q3_playtest.gd` sur main f82a03a6. Rapport :
- `docs/audit/q5-recette.md`. Branche `fix/q5-recette`, worktree `../gp-q5`.
- Réf. : commits f82a03a6

### q6-diplomatie.md — WIP Q6 — diplomatie (recette 2026-09-28) (2026-09-29)
- Test `game/tests/q6_diplomacy_test.gd` (1920×1080 @1,25 ; 1280×720 @1,0 ; 1280×640 @1,0).
- « Proposer le traité » hors écran : la carte des relations gardait une taille minimale
- calculée avant que le reste du panneau grandisse (avis reçus, fiche) → panneau plus haut que
- Réf. : commits 57b180d9

### q6-panneau-lateral.md — WIP Q6 — boutons du panneau latéral inaccessibles (2026-09-28) (2026-09-29)
- Branche `fix/q6-recette`, worktree `../gp-q6`. Test : `game/tests/q6_side_panel_test.gd`
- (1920×1080 taille 1,25 = vue 1280×720 ; 1280×720 et 1280×640 taille 1,0).
- Largeur : la zone `SIDE_PANEL` mesure 384 px en vue 1280×720, mais le panneau de province

### q6-recette.md — WIP Q6 — recette « comme un joueur » (2026-09-28) (2026-09-29)
- Demande du joueur : tester le jeu et corriger les bugs. Pilote `game/tests/q3_playtest.gd` sur
- main 24824cee (France, 1280×720, 12 tours). Branche `fix/q6-recette`, worktree `../gp-q6`.
- smoke : 56 × « _free_wrapper: Cannot convert argument 1 » (enveloppe libérée avant l'appel
- Réf. : commits 24824cee

### q6-toasts.md — Q6 — avis (zone TOASTS) au-dessus des fenêtres, repli du texte (2026-09-29)
- Test : `game/tests/q6_toasts_test.gd` (vert).
- Test rouge : registre des agents couvert par les avis/journal, « Colonies » aussi, journal et avis plus larges que la zone en vue étroite.
- Ordre d'affichage : zone TOASTS à l'étage HUD (`ui_layout.gd`), BANNER seulement pendant le bandeau de fin de tour (`map_ui.gd`) ; fenêtre bloquante hors pile = étage PANEL (`panel_stack.gd`).

### q7-recette.md — WIP Q7 — recette « comme un joueur » (2026-09-30) (2026-09-30)
- Demande du joueur : tester une partie et corriger les bugs. Pilote `game/tests/q3_playtest.gd`
- (France, 1920×1080 → vue 1280×720, 12 tours). Branche `fix/q7-recette`, worktree `../gp-q7`.
- PLANTAGE à l'ouverture de l'écran des factions en vue 1280×720 (« Message queue out of

### q8-recette.md — WIP Q8 — recette « comme un joueur » (2026-09-30) (2026-09-30)
- Demande du joueur : tester le jeu comme un joueur et corriger les bugs (budget 100 captures).
- Pilote `game/tests/q3_playtest.gd` : partie 1 Angleterre (toutes phases, 10 saisons), partie 2
- Bourgogne (naval, commerce, diplomatie, tours, sauvegarde, réglages).

## Grappe `r`

### r1-relief-campagne.md — R1 — Relief et occupation du sol réalistes de la carte de campagne (2026-09-25)
- Branche de worktree `agent-a75fccf335fda0e44` (depuis `integration/night`). ADR :
- `docs/decisions/0019-relief-et-occupation-du-sol.md`. Pipeline : `docs/geo.md` (section lot R1).
- **Données hors ligne** (`tools/cent_ans_tools/geo/`, tests `tools/tests/test_landcover.py`) :
- Réf. : ADR 0019

### r2-relief-bataille.md — Lot R2 — Relief réaliste des champs de bataille — terminé (non fusionné) (2026-09-25)
- Branche : `worktree-agent-a3ec0644f24b4ad29`. ADR : `docs/decisions/0020-relief-des-champs-de-bataille.md`.
- Cœur `core/crates/sim-battle/src/relief.rs` : flux dérivé `RELIEF_STREAM` (tirages d'avant R2
- inchangés), `ReliefStyle::of(terrain)` ; fBm à déformation de domaine + bruit « ridged » (crêtes,
- Réf. : ADR 0020

### r2b-ia-relief.md — Lot R2b — l'IA de bataille lit le relief — terminé (non fusionné dans main) (2026-09-25)
- Branche : `worktree-agent-a0839ed779ec029d6`. Suite de R2 (`docs/wip/r2-relief-bataille.md`, ADR 0020).
- `cd core && cargo test --release -p sim-battle --test ai_relief -- --ignored --nocapture`
- (`survey_active_against_passive` : IA active contre camp passif, armées miroir, graines 0-15 × 2 camps
- Réf. : ADR 0020 ; commits c5eb8861, 57032d45, 2e428a26

### r3-navgrid-forets.md — R3 — forêts historiques (R1) dans la grille de navigation (2026-09-25)
- Branche : `worktree-agent-aa0218e94ad47cabc` (depuis main afd327d4). ADR : `docs/decisions/0045-forets-historiques-dans-la-grille.md`
- (chiffres avant/après dans l'ADR).
- Squelette (ce fichier, ADR stub)
- Réf. : ADR 0019, 0045 ; commits afd327d4, cb1b4416

### r4-couvert-crete.md — Lot R4 — contre-pente contre l'arc long, position « couvert × crête » (2026-09-25)
- Branche `worktree-agent-a87b717776dc51d1c`. Suite de R2b (`docs/wip/r2b-ia-relief.md`), ADR 0046.
- Mesure R2b, graines 0-63 : plaine 113/128, bocage 89/128, collines 98/128, montagne 82/128 (total 382/512).
- Squelette : `data/rules/missile_arc.json` + schéma + test Python ; `src/missile_arc.rs`
- Réf. : ADR 0046 ; commits afd327d4, c47a54ce, 848119b5

## Grappe `rl`

### rl1-release.md — RL1 — build de release et performance globale (2026-09-25)
- Agent RL1, nuit du 25/09. Machine : Apple M4 Pro (Apple9), 48 Go, 14 cœurs, macOS 26, écran
- Retina (échelle 2). **Machine partagée** : charge moyenne 5 à 112 pendant les mesures, et surtout
- un autre Godot en rendu 1920×1080 (playtest Q1/Q2) une bonne partie du temps : le GPU est

## Grappe `sg`

### sg1-sieges.md — SG1 — Batailles de siège à la Total War (2026-09-25)
- Branche `worktree-agent-a806226fd4b8b61a9`. Backlog `docs/audit/backlog-tw.md` (Bataille : sièges).
- Assauts spectaculaires et lisibles : échelles dressées et escaladées, beffrois accostés (pont-levis
- abaissé), bélier sous manteau qui frappe en rythme, trébuchets et bombardes (projectiles, traînée,
- Réf. : ADR 0023 ; commits a8e1cc7a

### sg2-engins.md — SG2 — Engins de siège animés, huile, démos d'Avignon et de Bruges (2026-09-25)
- Branche `worktree-agent-a89b4bf8d49010116`. Suite de SG1 (`docs/wip/sg1-sieges.md`, ADR 0023,
- section « Suite SG2 ») et L3 (ADR 0026). Captures : `docs/audit/captures/sg2/`.
- Cœur : `Unit::reload_period`, `shot::ENGINE_RELOAD` ; pont `get_units().reload /
- Réf. : ADR 0023, 0026

### sg3-sieges.md — SG3 — Finitions de siège (servants, bombarde, LOD des engins, équilibre de l'assaut) (2026-09-25)
- Branche `worktree-agent-adaa478392d02c5a8`. Suite de SG2 (`docs/wip/sg2-engins.md`, ADR 0023).
- Sonde d'assaut `core/crates/sim-campaign/tests/sg3_assault_probe.rs` (Paris, Avignon, Bruges,
- Calais, Rouen × 10 graines, IA des deux côtés) ; tests non ignorés : porte d'Avignon (graine 11)
- Réf. : ADR 0023

### sg4-assaut.md — SG4 — IA d'assaut de siège et équilibre défenseur / attaquant à l'échelle épique (2026-09-26)
- Branche `worktree-agent-af1d6ff622ee45890` (worktree `agent-af1d6ff622ee45890`). Suite de SG3
- (`docs/wip/sg3-sieges.md`), ADR 0023 (SG2, SG3, **SG4**), 0076 et 0032-0034 (EP), 0046 (R4, **SG4**).
- Mesure de départ : sonde SG3 avec et sans engins ; matrice épique `sg4_balance`.
- Réf. : ADR 0023, 0046 ; commits b4bc9707

### sg5-crete-archers.md — SG5 — les tireurs et la cavalerie du défenseur ne rompent plus leur propre ligne sur la crête (2026-09-26)
- Branche `worktree-agent-add3a9bbd7d238220`. Suite de SG4 (`docs/wip/sg4-assaut.md`), ADR 0046
- (§ Suite SG4, **§ Suite SG5**), ADR 0056 (§ EP9b).
- `git merge main` (EP9b inclus).
- Réf. : ADR 0046, 0056

## Grappe `sm`

### sm1-smoke.md — SM1 — smoke de main réparé (2026-09-25)
- État : les deux régressions sont corrigées ; smoke vert dans le worktree (28 « smoke OK », code 0).
- Bisect (smoke comme juge, bornes b84344d2 bon / 848119b5 mauvais) : premier commit fautif
- **6efd3c84** `wip(ui1): textured parchment theme…` (UI1, interface enluminée).
- Réf. : commits b84344d2, 848119b5, 6efd3c84

### sm1-smoke-file-messages.md — SM1 — smoke : « Message queue out of memory » (2026-09-25)
- Symptôme : `smoke.gd` sortait en 138 après 4 fins de tour enchaînées (étape campagne),
- « Message queue out of memory » (`CanvasItem::_redraw_callback`).
- Cause : boucle de mise en page dans `DiplomacyPanel._fit_minimap` (connecté à
- Réf. : commits 6efd3c84

## Grappe `sv`

### sv2-unit-resources.md — SV2 — coûts en ressources des unités (2026-09-25)
- Branche `sv2-unit-resources`. Réutilise le mécanisme B7c (ADR 0053) des chantiers.
- Au recrutement, `cost.resources` de l'unité est tiré de l'offre libre de la faction
- (`free_supply`, une unité par province productrice accessible) ; le manque est importé à
- Réf. : ADR 0053

### sv4-ui-numbers.md — SV4 — chiffres de règles écrits en dur dans l'UI (2026-09-25) [restes]
- Branche `sv4-ui-numbers` (worktree, non fusionnée). Origine : `docs/wip/suites-bulles3.md`.
- Audit des GDScript (liste ci-dessous)
- Constantes de ravitaillement de `economy.rs` → `data/rules/economy.json` (+ schéma, défauts `EconomyRules`)

## Grappe `t`

### t2-perf.md — T2 / T8 / dette — perf carte, banc de bataille, tests (agent T2) (2026-09-25) [restes]
- Reprend l'audit `docs/audit/a5-technique.md` § 5, lots T2 et T8, plus une partie de la dette
- (T11/T12 ciblée : `test_portraits.py`, avertissements RGBFloat).
- `git merge main` (commit `f74dc4bb`) apportait V3 (mesure GPU/CPU du banc,
- Réf. : commits f74dc4bb

## Grappe `tw`

### tw.md — WIP orchestrateur — rapprochement Total War (session 6) (2026-09-25)
- Mandat (24/09) : autonomie complète, plusieurs heures. Rapprocher le jeu de Total War, visuellement et en mécaniques ; garder les mécaniques propres à Cent Ans.
- Hors périmètre : refonte colonies (C2c-C7, autre session, `docs/wip/colonies.md`), portraits.
- Choix du joueur au lancement :
- Réf. : ADR 0006, 0009 ; commits 92ed8a4c, 6ebe9658, 321222ae

### tw2-sb.md — TW2 lot SB — lisibilité et rythme de la destruction en siège (2026-09-28)
- Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § SB. Branche `feat/tw2-sb`. ADR 0107.
- Sonde `core/crates/sim-battle/tests/sb_siege_pace.rs` (tableau par niveau, tests de cibles niveau 3 / niveau 5).
- Données `data/rules/siege_works.json` retouchées (mur 100+200/niv., porte 40+70/niv.).
- Réf. : ADR 0107

### tw2-t1.md — TW2-T1 — Sort de la ville prise (2026-09-28)
- Branche `feat/tw2-t1`. Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T1. ADR 0101.
- Données : `data/rules/capture.json`, schéma `data/schemas/capture_rules.schema.json`, test
- `tools/tests/test_capture_rules_schema.py`.
- Réf. : ADR 0101

### tw2-t2.md — TW2-T2 — Reconstitution des armées et réserves de recrutement (2026-09-28)
- Branche `feat/tw2-t2`. Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T2. ADR 0102.
- Aucune reconstitution d'armée de campagne n'existait : seules les garnisons regagnent des hommes
- (`economy.rs`, effet `garrison`), et les blessés soignés après bataille (`medicine.rs`, H4).
- Réf. : ADR 0102

### tw2-t3.md — TW2-T3 — Compagnies de mercenaires (2026-09-28) [restes]
- Branche `feat/tw2-t3` (depuis `integration/tw2`, SB fusionné). Spec :
- `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T3. ADR 0103.
- Données : `data/rules/mercenaries.json` + schéma `mercenary_rules.schema.json` + test pytest
- Réf. : ADR 0103 ; commits 96ca4112
- Restes : La surprime n'apparaît pas dans le budget de l'interface (`economy.rs`, lot RS) : panneau et journal. / Pas d'illustration propre pour les deux nouvelles unités (icônes recadrées). / Pistes : compagnies allemandes en Italie, gallowglass, licenciement volontaire d'une compagnie.

### tw2-t5.md — TW2-T5 — Traditions d'armée (2026-09-28) [restes]
- Branche `feat/tw2-t5` (worktree `../gp-tw2-t5`, base `integration/tw2`). Spec :
- `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T5. ADR 0112.
- `CARGO_TARGET_DIR=core/target-t5`.
- Réf. : ADR 0112

### tw2-t4.md — TW2 lot T4 — points de capture en siège (bataille) et rééquilibrage des assauts (2026-09-29)
- Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T4. Branche `feat/tw2-t4` (depuis
- `feat/tw2-sb`). ADR **0108** (0104 pris ailleurs ; SB renuméroté 0107).
- Règles `data/rules/siege_capture.json` + schéma + pytest ; cœur `sim-battle/src/capture.rs`,
- Réf. : ADR 0108 ; commits 9145808e

## Grappe `u`

### u1-bogues-ui.md — U1 — bogues d'interface (lots U0 et U2 de l'audit A3) (2026-09-25)
- Branche : `worktree-agent-ac3b609abdc259c3f`. Source : `docs/audit/a3-ui.md` § 3 et § 8.
- Captures de vérification : `docs/audit/captures/u1/`.
- 1. `core/build.sh` puis `godot --headless --path game --import`.

## Grappe `ub`

### ub1-interface-bataille.md — UB1 — interface de bataille à la Total War (avant, pendant, après) (2026-09-25) [restes]
- Agent UB1 (session 7). Branche `worktree-agent-a17058e0e6f7d70f2`, partie de `integration/night`
- (fusion rapide pour disposer d'AU1 : bus Interface, `SoundBank`).
- Sources : `docs/audit/a3-ui.md` (U9, U13, défauts B1-B7), `docs/audit/backlog-tw.md`,
- Réf. : commits 506550a2
- Restes : Prévision d'équilibre = estimation simple (mêmes pièces que l'auto-résolution actuelle, sans / Retraite avant bataille : seul l'assaillant peut refuser (−10 moral) ; un assaut remis garde le / Butin : aucune règle de butin en bataille rangée ; l'encart montre les rançons à percevoir.

## Grappe `ur`

### ur1-unites.md — WIP — UR1 : variété des unités (roster à la Medieval II) (2026-09-25)
- Branche : `worktree-agent-a213f64dfca3b3a18`.
- Retouches hors nouveaux types : hommes d'armes à pied 950 / 90 → **1 050 / 100** (81-83 % sinon),
- piquiers flamands 600 / 55 → **650 / 60** (85 % sinon), sergents montés 750 / 70 → **700 / 65**
- Réf. : commits 8c058417, 5d5edd2d, 9668931a

## Grappe `ux`

### ux-prise-en-main.md — UX — prise en main (session du 25/09, « rendre l'interface plus intuitive ») (2026-09-25)
- Demande du joueur : « essaie de rendre l'interface utilisateur plus intuitive pour le joueur ».
- Point de départ : audit `docs/audit/a3-ui.md` ; U0-U5 et U7-U13 sont faits (UI2, UI3, UB1).
- Restent U6 (en partie fait par C7b), U14 (étiquettes) et U15 (tutoriel), plus des défauts
- Réf. : commits aaabb61c, 59113eb2

### ux1-carte-lisible.md — UX1 — Carte lisible (audit A3 : C8, C14, U14) (2026-09-25) [restes]
- Branche : `worktree-agent-a90f2668279e9e81c`. Plan d'ensemble : `docs/wip/ux-prise-en-main.md`.
- 1. Plaques d'effectif d'armée décalées hors des noms de ville et des autres plaques
- (`game/scripts/map/label_placer.gd` : 10 positions candidates, grille spatiale de 64 px,

### ux2-premiers-pas.md — UX2 — Premiers pas (barre du haut, tutoriel U15, conseil « que faire maintenant ») (2026-09-25)
- Branche : `worktree-agent-a109e2bd8dafc923f`. Plan d'ensemble : `docs/wip/ux-prise-en-main.md`.
- Audit : `docs/audit/a3-ui.md` (C9, C13, U1, U2, lot U15). Captures : `docs/audit/captures/ux2/`.
- Barre du haut : `game/scripts/map/map_ui.gd` (section « UX2 : libellés de la barre »),

## Grappe `zg`

### zg1-pyramide.md — ZG1 — pyramide de relief, paliers 1-2 (E1-E4) (2026-09-25)
- ADR 0036 (section « Mise en œuvre des paliers 1-2 »). Commande :
- `uv run --project tools cent-ans geo pyramid [--levels 1,2,3,4] [--force] [--workers N] [--limit N]`.
- Branche : `worktree-agent-a0df650280bb11446`. Doc : `docs/geo.md` (dernière section).
- Réf. : ADR 0036

### zg2-quadtree.md — ZG2 — moteur de relief streamé en quadtree (ADR 0036) (2026-09-25) [restes]
- Branche `zg2-quadtree` (worktree agent). Liens symboliques non versionnés : `data/map/pyramid`,
- `tools/geo/raw` → dépôt principal.
- `ReliefPyramid` : manifeste (RLE → ensembles de tuiles par étage), E0 = `data/map/height/`.
- Réf. : ADR 0036

### zg3-palier3.md — ZG3 — relief palier 3 (E5-E7) sur les zones de détail (2026-09-25) [restes]
- Lot ZG3 de l'ADR 0036. Branche `worktree-agent-a17b7688304a10059`.
- Commande : `uv run --project tools cent-ans geo detail-dem [--zones id,...] [--force]`.
- Doc : `docs/geo.md` § « Relief palier 3 ». Crédits : `CREDITS.md`.
- Réf. : ADR 0019, 0036
- Restes : Le plancher `MIN_LAND_M` reste une valeur plate (0,5 m) là où le rehaussement creuse fort / Le seuil `LAND_GAP_ALERT_M` (5 m) déclenche sur la quasi-totalité des zones : attendu vu leur

### zg4b-correctifs.md — ZG4b — correctifs de la vue rapprochée relevés en recette (Q3) (2026-09-25)
- Worktree d'agent (depuis `main`, refusionné après ZG6 7f38c532 puis MF1 / d80007a9). Liens symboliques
- non versionnés `data/map/pyramid`, `tools/geo/raw`. Dylib : `CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target`.
- Rendu seulement. Captures : `godot --path game res://scenes/start_menu.tscn -- --autostart=fac_england
- Réf. : ADR 0036 ; commits 7f38c532, d80007a9

### zg5a-hydro-fine.md — ZG5a — hydrographie fine, ancrages et routes drapées (données) (2026-09-25)
- Branche `worktree-agent-a2168a2690056db0a`, **rebasée sur `integration/zoom`** (3beab01c, ZG1+ZG3
- fusionnés : code de la pyramide et manifeste rempli). Cache partagé par liens symboliques
- (`tools/geo/raw`, `data/map/pyramid`, jamais commités). Coût : 0 $.
- Réf. : commits 3beab01c

### zg5b-rendu-fin.md — ZG5b — rendu de l'hydrographie fine, des routes drapées, des ancrages et du parcellaire de près (2026-09-25)
- Branche `worktree-agent-a1d8f6f50c1e29cce` (depuis `main` 5a33ec1f ; `main` refusionné après ZG4,
- PF1, PB1, puis epic + bulles3 le 2026-09-25 ; tests zg2, zg5b, smoke verts après fusion). Cache partagé par liens symboliques non versionnés `data/map/pyramid`, `tools/geo/raw`.
- ADR 0036, contrat `docs/geo.md` § « Hydrographie fine ». Doc : `docs/godot-map.md` § « Hydrographie
- Réf. : ADR 0036 ; commits 5a33ec1f

### zg6-villes.md — ZG6 — villes ordinaires à l'échelle réelle vers 1340 (ADR 0036) (2026-09-25)
- Branche `worktree-agent-a8f477a463e9704f3` (worktree d'agent, depuis `main` 08eecdd8).
- Liens symboliques non versionnés : `data/map/pyramid`, `tools/geo/raw` → dépôt principal.
- Dylib : `CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target cargo build -p godot-bridge`
- Réf. : ADR 0036 ; commits 08eecdd8

### zg8-relief-exagere.md — ZG8 — relief exagéré façon Total War (visuel seulement) (2026-09-25)
- État : TERMINÉ (branche worktree-agent-a324124257f1dc359, main fusionnée), à fusionner dans main.
- Squelette : `ReliefExaggerationProfile` (+ `resources/relief_exaggeration.tres`), `ReliefFloor`
- (fond min+flou, WorkerThreadPool), `MapData.display_height` / `height_from_display` / `set_relief_floor`,
- Réf. : ADR 0036

### zg-zoom-geographique.md — ZG — carte de campagne zoomable jusqu'à 1-5 m (ADR 0036) (2026-09-26)
- Demande du joueur (25/09) : « voir des montagnes, des vallées, des villes » en zoomant ; paliers
- 1 (90 m), 2 (30 m) et 3 (1-5 m) validés, « fais tout jusqu'à 3 sans me demander ».
- Session orchestratrice « zoom ». Fusion via le worktree `../gp-zoom-merge` (branche
- Réf. : ADR 0036, 0037, 0074 ; commits 3beab01c, 650d77a5, 26ba32fa

### zg7a-perf-finitions.md — ZG7a — perf et finitions visuelles de la vue rapprochée (ADR 0036) (2026-09-26) [restes]
- Worktree d'agent `worktree-agent-a81b596dc59312952` (depuis `main` 369bc6e7). Liens symboliques non
- versionnés : `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib :
- `CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target cargo build -p godot-bridge` puis copie
- Réf. : ADR 0036 ; commits 369bc6e7, fecf87ec
- Restes : Le relief E1-E4 (ZG1) plaque les fonds de vallée proches de plateaux à 0,5 m (rehaussement de / Tamise fine : niveau d'eau -7,8 m à Londres (PAVA mêlé à la bathymétrie de l'estuaire).

## Grappe `bulles`

### bulles-partout.md — WIP — Bulles partout (session historien, 25/09) (2026-09-26)
- Conception : `docs/design/2026-09-25-bulles-partout.md`.
- Fusion : worktree `../gp-historien-merge` (branche `integration/historien`) puis ff-only dans main.
- Commits avec chemins explicites (index partagé avec d'autres sessions).
- Réf. : ADR 0053 ; commits a51353d8, 3ec20811

## Grappe `ct`

### ct1-tour-ia-camera.md — CT1 — tour de l'IA à la Total War : marches visibles, caméra qui suit, vitesse (état) (2026-09-26)
- Branche : `ct1-ai-turn-replay` (worktree `agent-ac735a99732c7b4b8`), partie de main `168b1acf`. ADR : `docs/decisions/0073-relecture-du-tour-ia.md`.
- Cœur : `sim-campaign/src/ai_replay.rs` (`AiMoveRecord`, `AiMoveKind`, `AiMoveNotability`, `CampaignState::set_ai_replay_recording`, `ai_turn_moves`), branché dans `turn.rs` (`continue_ai_marches`, `apply_ai_order`, `ai_replay_beg…
- Pont : `campaign_sim_ai_replay.rs` (`set_ai_turn_recording`, `get_ai_turn_moves`).
- Réf. : ADR 0073 ; commits 168b1acf, b04b88ce

## Grappe `dc`

### dc-densite.md — DC — Carte plus dense et plus lente (2026-09-26)
- Demande du joueur (2026-09-26) : « la carte est trop petite / les armées vont trop vite », « plus de
- villes pour une région donnée » ; accord sur B + C, « tu fais le plan ET les modifications ».
- ADR 0082. Orchestrateur : session DC. Coût cloud : 0 $ (recherche et calcul locaux).
- Réf. : ADR 0082 ; commits 6f74c214, 159aff80, 848fff43

## Grappe `epic`

### epic.md — Batailles épiques (session du 25/09) (2026-09-26) [restes]
- Demande du joueur : rendre les batailles épiques. Carte de bataille réaliste et accidentée (collines,
- rivières, ponts, villages), horizon peint ou lointain hors de la zone jouable (mer, montagnes), unités
- massives portant chacune un étendard (une figurine porte-étendard), cris et fracas d'armes quand la
- Réf. : ADR 0022, 0031, 0033, 0035, 0055, 0056 ; commits 13506bf1, a30b461b, ade1b9a7
- Restes : Équilibrage : avec R4, le défenseur gagne à forces égales à l'échelle épique (graines 3, 5, 11) ; / `--standard-shot=foot` cadre parfois dans une pile de pont sur un site EP3 : décaler la caméra. / `battle_skinned_poses.py` : défauts ruff (docstrings) antérieurs, venus d'EP5.

## Grappe `fg`

### fg0-prototype.md — FG0 — Prototype et planche de style (figurines fines) (2026-09-26) [restes]
- Branche : `feat/fg0-prototype` (worktree agent). Plan : `docs/wip/fg-figurines-fines.md`.
- Planche : `docs/img/fg/planche_fg0.png` (+ images séparées dans `docs/img/fg/`).
- Base humaine : MakeHuman via MPFB 2.0.17 (données CC0, code GPL installé hors dépôt) ;

### fg1-corps.md — FG1 — Corps humain des figurines fines (MakeHuman en production) (2026-09-26) [restes]
- Branche : `feat/fg1-corps` (worktree agent). Plan : `docs/wip/fg-figurines-fines.md`.
- Prototype : `docs/wip/fg0-prototype.md`.
- Rig aux proportions réalistes (`battle_fine_rig.py`), rigs `human` / `cavalry` recuits

### fg2-equipement.md — FG2 — Équipement fin des figurines de bataille (2026-09-26)
- Branche : `feat/fg2-equipment` (worktree agent). Plan : `docs/wip/fg-figurines-fines.md`.
- Précédents : `docs/wip/fg1-corps.md`, `docs/wip/fg0-prototype.md`. main (FG4, 0a53488e)
- fusionné dans la branche le 26/09 ; `rigs`, `figures` (28) et `check` relancés après.
- Réf. : commits 0a53488e

### fg3-matieres.md — FG3 — Matières cuites des figurines fines (2026-09-26) [restes]
- Branche : `feat/fg3-materials` (worktree agent `agent-aba08b13133c215dc`). Plan :
- `docs/wip/fg-figurines-fines.md`. Précédents : `fg0-prototype.md` (cuisson test),
- `fg1-corps.md`, `fg2-equipement.md`, `fg4-cheval.md`.
- Réf. : ADR 0088 ; commits 4c89ce24

### fg4-cheval.md — FG4 — Cheval fin en production (2026-09-26) [restes]
- Branche : `feat/fg4-horse` (worktree agent, `main` et FG1 intégrés, non fusionnée dans `main`). Plan : `docs/wip/fg-figurines-fines.md`.
- Prototype : `docs/wip/fg0-prototype.md`.
- Déformation : ajustement **articulation par articulation** (`battle_fine_horse.OGA_JOINTS`)
- Réf. : commits 15ade06c

### fg5-perf.md — FG5 — Performance et bascule des figurines fines par défaut (2026-09-26)
- Branche : `feat/fg5-perf` (worktree `.claude/worktrees/fg5-perf`). Plan : `docs/wip/fg-figurines-fines.md`.
- Précédents : `fg1-corps.md`, `fg3-matieres.md`, ADR 0088. Décision : **ADR 0089**.
- Après fusion de main : smoke, `pb3c_buffers_test`, `pb3e_step_thread_test`, `bv3_check`, `fg3_maps_test` OK.
- Réf. : ADR 0088, 0089

## Grappe `fr`

### fr1-frontieres.md — FR1 — Frontières de faction lumineuses (façon Total War) (2026-09-26)
- Branche : `feat/fr1-faction-borders`. ADR : `docs/decisions/0074-frontieres-de-faction.md`.
- **État : terminé, prêt à fusionner** (main fusionnée dans la branche le 2026-09-26).
- Crochet dans le fragment du terrain : `game/shaders/faction_borders.gdshaderinc`
- Réf. : ADR 0074

## Grappe `liste`

### liste-unites.md — Liste « Mes unités » (touche U) (2026-09-26) [restes]
- Branche `feat/unit-roster`, worktree `../gp-roster`.
- Dérouler la liste de ses unités sur la carte de campagne (armées, agents), avec leur portée de
- déplacement ; un clic mène à l'unité.

## Grappe `nuit`

### nuit.md — WIP orchestrateur — session 7 (nuit du 24/09) : Cent Ans moderne et de grande qualité (2026-09-26)
- Mandat : autonomie complète toute la nuit. Inspiration principale Total War. Tous les aspects : graphismes, assets, UI, mécaniques, équilibre, animations, modèles 3D, physique, audio, performance.
- Orchestrateur unique : reprend la session 6 (TW, `docs/wip/tw.md`, vague 6 C4/C5/B8) et le mouvement libre (`docs/wip/mouvement-libre.md`, M1-M5). Les autres sessions sont fermées par le joueur.
- Budget : **nouvelle enveloppe de 50 $** propre à cette session (section « Session 7 » de `docs/budget.md`).
- Réf. : ADR 0014, 0015, 0016, 0019, 0020, 0021 ; commits 5edf3938, af78e73b, 58733bb2

## Grappe `pb`

### pb1-benchmark-perf.md — PB1 — Benchmark et optimisation des performances (FPS, chargements, zooms) (2026-09-26)
- Demande (2026-09-25) : benchmarker le jeu et optimiser FPS + temps d'exécution (chargements,
- zooms) sans dégrader le contenu. **État : fusionné dans main** (f2eacfb1 ombres, puis commit
- `perf(map): PB1 …`). ADR 0051. Coût cloud : 0 $.
- Réf. : ADR 0051, 0062 ; commits f2eacfb1

### pb2-vegetation-shader.md — PB2 — Semis natif de la végétation + shader du terrain (2026-09-26)
- Demande (2026-09-25) : pistes 1 et 2 de PB1 uniquement : (1) porter le semis des tuiles de
- végétation en Rust, (2) alléger le shader du terrain de près sans changement visible.
- Worktree `/Users/jean_hubert/dev/gp-pb2`, branche `pb2-veg-shader`. Coût cloud : 0 $.
- Réf. : ADR 0062

### pb3-performance.md — PB3 — Rust et M4 Pro : FPS et vitesse d'exécution (2026-09-26) [restes]
- Demande du joueur (2026-09-26) : « quelle utilisation efficace de Rust et du M4 pour améliorer les
- performances (FPS et rapidité d'exécution) » ; accord sur tout le plan (« ok pour tout »).
- Orchestrateur : session PB3 (checkout principal). Coût cloud : 0 $ (calcul local).
- Réf. : ADR 0079, 0080, 0081, 0090, 0091, 0092 ; commits a07458c5, 8955224f, 2ccea813

### pb3b-metalfx.md — PB3b — Mise à l'échelle 3D MetalFX (2026-09-26)
- Lot de PB3 (`pb3-performance.md`). Branche `worktree-agent-ab0e3b517d25d7cf4`. ADR 0080.
- Rendu seulement (rien dans `core/`).
- `RenderQuality` : clés `upscale_mode` / `upscale_scale`, `apply_upscale` (MetalFX sous Metal,
- Réf. : ADR 0080

### pb3c-bataille-cpu.md — PB3c — CPU par image en bataille (2026-09-26) [restes]
- Branche `perf/pb3c-battle-cpu` (worktree d'agent). Plan : `docs/wip/pb3-performance.md`.
- Objectif : moins de CPU par image en bataille, rendu identique, simulation inchangée.
- 1. Rust : cache des poses des figurines (clé : pas de sim, époque des mutations hors pas,
- Réf. : commits bb251b68, 6a837e99
- Restes : `get_units` et les tampons renvoyés entre deux pas sont partagés : lecture seule côté GDScript. / `_find_braced` laissé tel quel (déjà O(n) hors charge de cavalerie). / Pistes suivantes : `soldiers.update` (~3,3 ms, boucles GDScript), étendards 1,7 ms, audio

### pb3d-fin-de-tour.md — PB3d — fin de tour dans un fil, rafraîchissement groupé (2026-09-26) [restes]
- Branche `feat/pb3d-end-turn-thread` (worktree agent). Plan général : `docs/wip/pb3-performance.md`.
- ADR : `docs/decisions/0081-fin-de-tour-dans-un-fil.md`.
- Pont Rust : `turn_job.rs` (fil de fin de tour, `resolve_turn` commun sync/async, test
- Réf. : ADR 0081 ; commits 7cfef434, 6a837e99

### pb3f-rayon.md — PB3f — rayon : fin de tour parallèle, déterminisme gardé (2026-09-26) [restes]
- Branche `perf/pb3f-rayon` (worktree agent `agent-a27afc73c1888787a`). Plan général :
- `docs/wip/pb3-performance.md`. ADR : `docs/decisions/0091-parallelisme-de-la-fin-de-tour.md`.
- Profil release (graine 1, 20 tours, `sample` macOS) : planificateur ≈ 80 % du cœur,
- Réf. : ADR 0091 ; commits a7877ac6, d246c5c3

## Grappe `relecture`

### relecture-orleans.md — Relecture historique — Orléans 1:1 (VH7) (2026-09-26) [restes]
- Branche `feat/relecture-orleans` (worktree d'agent). Mission : relire les restitutions de
- `data/landmarks_v2/orleans.json` (faits à relire de `docs/wip/vh7-orleans.md`), corriger d'après
- des sources sérieuses, rapport `docs/histoire/relecture-vh-orleans.md`.

### relecture-vh-londres.md — Relecture historique — Londres vers 1340 (VH6) (2026-09-26) [restes]
- Branche : `worktree-agent-a50490ef8dc8e2554`. Rapport : `docs/histoire/relecture-vh-londres.md`.
- Lecture des données (`data/landmarks_v2/london.json`, `docs/wip/vh6-londres.md`)
- Recherche des sources (NHLE/Historic England via ArcGIS ouvert, VCH, Survey of London,

### relecture-vh-paris.md — Relecture historique de Paris vers 1340 (VH5) (2026-09-26)
- Branche : worktree d'agent `worktree-agent-a8c2568b3bd7b7929` (depuis `main`). Rapport :
- `docs/histoire/relecture-vh-paris.md`.
- Corrections dans `tools/geo/paris_v2_author.py`, puis

## Grappe `suites`

### suites-bulles3.md — Suites ouvertes de la vague 3 « bulles partout » (25/09) (2026-09-26)
- Origine : fin de `docs/wip/bulles-partout.md` (« Suites ouvertes »). Le joueur a demandé de tout corriger.
- Fusion : worktree `../gp-suites3-merge` (branche `integration/suites3`), puis ff-only dans main.
- L'IA voit toute la carte (choix M5a) ; maquettes, forêts, hameaux et mer non voilés.
- Réf. : ADR 0053 ; commits 35976b9c

## Grappe `sz`

### sz-suites-zoom.md — SZ — suites du zoom géographique (après clôture de ZG, ADR 0036) (2026-09-26) [restes]
- Demande du joueur (26/09) : traiter les 7 imperfections laissées par ZG7c, « en toute autonomie ».
- Orchestrateur : session `game-project-d8`. Coût cloud attendu : 0 $ (calcul local, données ouvertes).
- Référence des défauts : `docs/wip/zg7c-recette.md` (tableau « défauts laissés », captures
- Réf. : ADR 0036, 0077, 0078, 0079, 0086 ; commits f8ee130f, 80fb0004, 4b6c1057

### sz1-montagnes.md — SZ1 — haute montagne au palier vallée (défaut S1 de ZG7c) (2026-09-26)
- Branche `sz1-montagnes` (depuis `main`, worktree d'agent ; `main` fusionnée après SZ2). Liens
- symboliques non versionnés : `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib :
- `CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target` puis copie dans `game/bin/`.
- Réf. : ADR 0036

### sz2-fonds-de-vallee.md — SZ2 — fonds de vallée E0-E4 (défaut S2 de ZG7c) (2026-09-26)
- Chantier SZ (`docs/wip/sz-suites-zoom.md`). Branche `feat/sz2-valley-floors` (worktree d'agent
- `agent-acc401f3ef1c1478d`, depuis `main` 7e1ac032). Agent « données » : pas de compilation Rust.
- `data/map/pyramid` du worktree → lien vers `/Users/jean_hubert/dev/game_project/data/map/pyramid.sz2`
- Réf. : ADR 0019 ; commits 7e1ac032

### sz2b-nappe-fleuves.md — SZ2b — nappe d'eau des fleuves au palier site (suite de SZ2) (2026-09-26)
- Chantier SZ (`docs/wip/sz-suites-zoom.md`). Branche `sz2b-water-sheet` (worktree d'agent
- `agent-a098d00691a342944`, depuis `main` 89bc960a). Liens symboliques non versionnés
- `data/map/pyramid`, `tools/geo/raw` → dépôt principal ; dylib reconstruite
- Réf. : ADR 0086 ; commits 89bc960a

### sz4-objets-echelle.md — SZ4 — objets à l'échelle aux paliers intermédiaires, disque d'emprise des villes (suites ZG7c S4, S5) (2026-09-26)
- Branche `feat/sz4-objets-echelle` (worktree d'agent, depuis `main` 7e1ac032). Liens symboliques non
- versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib copiée de
- `/Users/jean_hubert/dev/game_project/game/bin/` (aucun changement Rust). Rendu seulement.
- Réf. : commits 7e1ac032

### sz4b-colonies-forets.md — SZ4b — maquettes de colonies continues, forêts denses au palier vallée (suites SZ4) (2026-09-26) [restes]
- Branche `feat/sz4b-colonies-forets` (worktree d'agent, depuis `main` 01f09d2b). Liens symboliques non
- versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Compilation avec
- une cible **privée** `CARGO_TARGET_DIR=<worktree>/core/target` (à supprimer après fusion), dylib copiée
- Réf. : commits 01f09d2b, 60061b0b

### sz5-pluie.md — SZ5 — pluie crédible à tous les paliers de zoom (défaut S7) (2026-09-26)
- Worktree `agent-adeddc2012349e482` (depuis `main`). Lien : `docs/wip/sz-suites-zoom.md`,
- `docs/wip/zg7c-recette.md` (défaut S7 : « pluie en bâtonnets blancs géants au palier site »).
- `CampaignWeatherView._update_particles` dimensionnait les gouttes avec

### sz6-pics-scripts.md — SZ6 — pics d'images côté scripts sur la carte de campagne (suite S6 de ZG7c) (2026-09-26)
- Branche `feat/sz6-script-spikes` (worktree d'agent, depuis `main` 7e1ac032, `main` c4064c29
- fusionné). Liens symboliques non versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal.
- Objectif : p99 des images < 50 ms sur le parcours `--bench-map`, sans changement visuel. **Atteint.**
- Réf. : ADR 0051 ; commits 7e1ac032, c4064c29

## Grappe `vh`

### vh-villes-historiques.md — VH — Villes historiques et peuplement à l'échelle du zoom géographique (2026-09-26) [restes]
- Chantier **suivant** ZG (ADR 0036, `docs/wip/zg-zoom-geographique.md`). Demandé par le joueur le
- 2026-09-25 : « une fois [ZG] terminé, revoir les forêts, villes etc. qui peuplent la carte à cette
- nouvelle échelle ; créer des représentations réalistes des villes de l'époque (Paris, Londres,
- Réf. : ADR 0015, 0026, 0036, 0037

### vh4-villes-emblematiques.md — VH0 + VH4 — villes emblématiques 1:1 géoréférencées (ADR 0078) (2026-09-26) [restes]
- Branche `feat/vh4-landmarks-1to1` (worktree d'agent, depuis `main` 7e1ac032). Orchestration SZ
- (`docs/wip/sz-suites-zoom.md`), défaut S3 de ZG7c. Liens symboliques non versionnés
- `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib :
- Réf. : ADR 0036, 0078 ; commits 7e1ac032, ac568e3b

### vh5-paris.md — VH5 — Paris vers 1340 à l'échelle 1:1 (format landmark v2, ADR 0078) (2026-09-26)
- Branche `feat/vh5-paris` (worktree d'agent, depuis `main` 89bc960a). Orchestration SZ
- (`docs/wip/sz-suites-zoom.md`), chantier VH (`docs/wip/vh-villes-historiques.md`). Référence :
- `docs/landmarks-v2.md` (section « Paris et ALPAGE »), addendum VH5 de l'ADR 0078. Liens
- Réf. : ADR 0078 ; commits 89bc960a

### vh6-londres.md — VH6 — Londres vers 1340 à l'échelle 1:1 (format `landmark` v2, ADR 0078) (2026-09-26) [restes]
- Branche `feat/vh6-london` (worktree d'agent, depuis `main` 89bc960a). Orchestration SZ
- (`docs/wip/sz-suites-zoom.md`), chantier VH (`docs/wip/vh-villes-historiques.md`, lot VH6).
- Référence du format : `docs/landmarks-v2.md` ; exemple : `data/landmarks_v2/rouen.json`.
- Réf. : ADR 0078 ; commits 89bc960a
- Restes : Tracé du mur entre Newgate et Aldersgate et position d'Aldersgate ; point de départ à la Tour. / Position et axe d'Old St Paul's (−4° grille, centre 15 m à l'est de la cathédrale de Wren), / London Bridge : extrémités (à l'est de St Magnus), hauteur du tablier (5,5 m au-dessus de

### vh7-orleans.md — VH7 — Orléans vers 1340-1429 à l'échelle 1:1 (format v2, ADR 0078) (2026-09-26) [restes]
- Branche `feat/vh7-orleans` (worktree d'agent, depuis `main` 89bc960a). Orchestration SZ
- (`docs/wip/sz-suites-zoom.md`), chantier VH (`docs/wip/vh-villes-historiques.md`, lot VH7).
- Référence : `docs/landmarks-v2.md` (section « Orléans vers 1340-1429 »), exemple Rouen. Liens
- Réf. : ADR 0026, 0078 ; commits 89bc960a
- Restes : **Butte** : le relief réel monte de 20-25 m entre la Loire (87 m) et la cathédrale (115 m, RGE / **Loire sans nappe d'eau** au palier site (SZ2b en cours) : le pont franchit un pré ; les quais et / Entre le mur de Loire (47,8983 N) et l'eau fine, une bande de 50-80 m (quais du XVIIIe s. gagnés

## Grappe `windows`

### windows-release-notes.md — Installer et jouer (2026-09-26)
- Première préversion jouable sous **Windows 10/11 64 bits** (ADR 0087).
- 1. Télécharger **`Cent.Ans.Windows.zip`** (≈ 770 Mo) ci-dessous.
- 2. Clic droit → **Extraire tout…** (ne pas lancer le jeu depuis l'intérieur du zip).
- Réf. : ADR 0087

## Grappe `cb`

### cb-m1-contours.md — CB-M1 — Contours de formation en décales (état) (2026-09-27)
- Branche : `feat/cb-m1-outline`. Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`
- (section CB-M1). Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`
- (« Contour de formation »).

### cb-m2-trajet-curseur.md — CB-M2 — Aperçu du trajet et curseur contextuel (état) (2026-09-27)
- Branche : `feat/cb-m2-path-hover`. Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`
- (section CB-M2, écart 1). Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`.
- Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cbm2`.

### cb-m3-file-ordres.md — CB-M3 — Ordres en file (état) (2026-09-27) [restes]
- Branche : `feat/cb-m3-queue` (depuis `main` 88ec35be, qui contient CB-M2). Plan :
- `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB-M3, écart 6).
- Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cbm3`.
- Réf. : ADR 0095 ; commits 88ec35be

### cb-m4-portee-comparaison.md — CB-M4 — Portée au sol et comparaison au survol (état) (2026-09-27) [restes]
- Branche : `feat/cb-m4-range-compare` (partie de `main` 88ec35be, qui contient CB-M2 : la tête
- initiale du worktree, be631979, précédait CB-M2 et n'avait pas `hover_context`).
- Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB-M4).
- Réf. : commits 88ec35be, be631979

### cb0-entrees.md — CB0 — Extraction des entrées de bataille (état) (2026-09-27)
- Branche : `feat/cb0-battle-input`. Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`
- (section CB0). Orchestration : `docs/wip/cb.md` (lecture seule pour cet agent).
- Squelette `game/scripts/battle/battle_input.gd` (signaux, pas encore utilisé par la scène).

### cb1-formation-glisser.md — CB1 — Formation au glisser et verrouillage de groupe (état) (2026-09-27) [restes]
- Branche : `feat/cb1-drag-formation` (depuis `main` b985d752). Plan :
- `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB1, écarts 5 et 6).
- Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cb1`.
- Réf. : ADR 0095 ; commits b985d752

### cb6-formations-groupe.md — CB6 — Formations de groupe (état) (2026-09-27) [restes]
- Branche : `feat/cb6-group-formations` (depuis `main` cad1ca05). Plan :
- `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB6 et décisions après relecture).
- Relecture historique : `docs/research/cb6-formations.md`.
- Réf. : ADR 0095 ; commits cad1ca05

### cb.md — CB — Contrôles de bataille façon Total War (orchestration) (2026-09-28) [restes]
- Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`.
- Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`. ADR réservée : `docs/decisions/0095-controles-bataille-tw.md`.
- Icônes : les agents de la vague 4 dessinent des glyphes en code ; la session principale génère toutes les icônes DA5 (curseurs, cadenas, modes, états, alertes, capacités) en une fois après CB4 (≈ 2 $, `docs/budget.md`).
- Réf. : ADR 0095 ; commits 135a7b36

### cb-icones.md — CB — icônes des contrôles de bataille (pipeline DA5) (2026-09-28)
- Branche `feat/cb-icons` (depuis main 6d581bb2). Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` § Icônes.
- Catalogue `data/ui/icons_ink.json` : 16 dessins nouveaux (groupe `cb`, préfixe `cb_`) + cibles ajoutées
- à 6 dessins existants (réemploi gratuit : `stance_normal` → garde, `martial` → mêlée, `enemy_army` →
- Réf. : commits 6d581bb2

### cb2-modes.md — CB2 — Modes d'unité, icônes d'état, remappage des touches (état) (2026-09-28) [restes]
- Branche : `feat/cb2-modes` (depuis `main` 220a1e9f). Plan :
- `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB2). Fusion **en dernier** de
- la vague 4 (après CB3, CB5, CB6). Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cb2`.
- Réf. : ADR 0095 ; commits 220a1e9f

### cb3-camera-vue-tactique.md — CB3 — Ralenti, caméra (lacet/tangage), vue tactique, `spotted` (2026-09-28) [restes]
- Branche `feat/cb3-camera-tactical`, depuis `main` (cad1ca05 / 220a1e9f). Plan :
- `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (§ Écarts point 2, Conventions
- communes, CB3). Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md` (§ CB3).
- Réf. : commits cad1ca05, 220a1e9f

### cb4-capacites.md — CB4 — Capacités actives (état) (2026-09-28)
- Branche : `feat/cb4-abilities` (depuis `main` 07eff33b, après « docs: CB wave 4 merged, CB4 next »).
- Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (§ CB4) ; relecture
- historique : `docs/research/cb4-capacites.md`. Cible cargo privée :
- Réf. : ADR 0095 ; commits 07eff33b

## Grappe `mf`

### mf1-filtres-carte.md — MF1 — Filtres de la carte de campagne (2026-09-27)
- Terminé le 25/09/2026 (ADR 0048). Branche `feat/map-modes`.
- Cœur : `sim_campaign::map_lens` (revenu, population, mécontentement, loyauté du vassal,
- ravitaillement, revendications), `economy::seasonal_supply_change` partagé avec
- Réf. : ADR 0048

## Grappe `po`

### po1-layout.md — PO1 — Disposition fixe (`feat/po1-layout`) (2026-09-27)
- Plan : `docs/superpowers/plans/2026-09-27-po-polish.md` § PO1. Bible DA § 12.1, ADR 0097.
- 1. `UiLayout` implémenté + tests « UiLayout » dans `po_ui_test.gd`
- 2. Migration carte de campagne (TOP_BAR, BOTTOM_SELECTION, MINIMAP, SIDE_PANEL, TOASTS, MODAL)
- Réf. : ADR 0097

### po3-map.md — PO3 — Carte de campagne (lumière, forêts, étiquettes) (2026-09-27)
- Branche `feat/po3-map` (depuis `feat/po-polish`, qui y est fusionnée : PO2, AN1a). Plan :
- `docs/superpowers/plans/2026-09-27-po-polish.md` § PO3. Références : bible DA § 12.6, ADR 0097.
- **État : fait, prêt à fusionner** (l'orchestrateur fusionne).
- Réf. : ADR 0097

### po5-motion.md — PO5 — Mouvement et transitions (wip) (2026-09-27)
- Branche `feat/po5-motion` (depuis `feat/po-polish`, qui y est fusionnée). Plan :
- `docs/superpowers/plans/2026-09-27-po-polish.md` § PO5.
- `SceneFader.go(path)` / `cover()` / `reveal()` (`game/scripts/ui/scene_fader.gd`, façade

## Grappe `proto`

### proto-moteur.md — Prototype comparatif Godot / Unreal (proto-moteur) (2026-09-27)
- Question du joueur (27/09) : « migrer sur un autre moteur rendrait-il le jeu plus beau ? »
- Réponse : prototype sur une toute petite section, mêmes assets et même caméra dans les deux
- moteurs, chacun poussé au maximum. Unreal est piloté par agent (MCP `runreal/unreal-mcp`, protocole
- Réf. : ADR 0093

## Grappe `dz`

### dz-frontieres-diplomatie.md — DZ — frontières diplomatiques (2026-09-28, nuit) (2026-09-28)
- Demande : en mode Diplomatie, frontières rouges pour les ennemis ; en sélectionnant une faction,
- voir ses ennemis / neutres / alliés.
- Cœur (pont) : `get_province_stances_for(viewer, ids)`, `get_faction_stances_for(viewer)`

## Grappe `fe`

### fe.md — FE — féodalité et petites factions (orchestration) (2026-09-28)
- Spec : `docs/superpowers/specs/2026-09-28-feodalite-design.md`. Plan : `docs/superpowers/plans/2026-09-28-feodalite.md`.
- ADR : 0098. Worktree orchestrateur : `../game_project-fe` sur `feat/fe`.
- **Vagues 1 et 2 dans `main`** (26bea252) : F0-F5 + registres F4a-F4e. 91 factions (dont `fac_rebels`),
- Réf. : ADR 0085, 0110, 0113, 0114 ; commits 26bea252, f8c126d5

### fe-heraldry.md — FE — moteur de blasons (heraldry.py), meubles manquants (2026-09-28)
- `uv run --project tools pytest -q tools/tests/test_heraldry.py tools/tests/test_heraldry_houses.py`
- passe (11 tests, dont un nouveau test de non-régression).
- Ajout au moteur (`tools/cent_ans_tools/heraldry.py`) de dessins simples pour les

### fe1-deductions.md — FE1 — déductions, obligations, loyauté (`feat/fe1-deductions`) (2026-09-28) [restes]
- Plan : `docs/superpowers/plans/2026-09-28-feodalite.md` § F1. ADR 0098.
- **F1 terminé** (branche `feat/fe1-deductions`), suites Rust et Python vertes hors points ci-dessous.
- `FactionState::suzerain` = vue en cache de `feudal::liege_of`, recalculée par
- Réf. : ADR 0098 ; commits dfe88244

### fe2-escalade.md — FE2 — escalade de guerre et guerre privée (2026-09-28)
- Branche `feat/fe2-escalade` (depuis `main` dfe88244). Spec § 4.3, plan F2.
- Règles : `feudal.json` / schéma / `FeudalRules::escalation` (`EscalationRules`, `ProtectionScore`,
- `feudal.rs` (ajouts en fin de fichier) : `adjust_loyalty`, `common_liege`, `protection_score`
- Réf. : commits dfe88244

### fe3-titres.md — FE3 — transferts de titres (lot F3 du chantier FE) (2026-09-28)
- Branche `feat/fe3-titres` (depuis `main` 1dcd6821). Plan : `docs/superpowers/plans/2026-09-28-feodalite.md` § F3.
- `sim-campaign/src/feudal.rs` : `FeudalState` étendu (`forfeitures`, `disputes`, `start_crowns`,
- `streaks`, `objectives_met`), stubs F0 remplis par délégation, `liege_of` (titre vacant = indépendance).
- Réf. : commits 1dcd6821

### fe4a-assets.md — FE4a — données dérivées après F4a (9 provinces, 7 factions) (2026-09-28)
- `uv run --project tools pytest -q` : 840 passed, 2 skipped (skips pré-existants, sans rapport).
- `cd core && cargo test -p data-model` : tout au vert.
- Armes de la maison d'Albret ajoutées à `data/heraldry/houses.json` (de gueules plain, historique, attesté,

### fe4a-france.md — FE4a — Registre féodal de la France (1337) (2026-09-28)
- Branche `feat/fe4a-france`, issue de `main` (dfe88244, qui contient FE0 1dcd6821).
- 9 nouvelles provinces créées (`data/provinces/`) : Alençon, Évreux, Albret, Charolais,
- Penthièvre, Blois, Foix, Armagnac, Valois. Graines/poids ajoutés pour `cent-ans geo provinces`.
- Réf. : commits dfe88244, 1dcd6821

### fe4b-c3-saxe.md — FE4b — grappe C3 « Saxe / Brandebourg / Thuringe » (2026-09-28)
- Sous-tâche du chantier FE4b (Empire et Pays-Bas), branche `feat/fe4b-empire`. Données uniquement,
- pas de commit, pas de `cargo test`, pas de pipeline géo (fait par l'agent orchestrateur).
- Provinces : `data/provinces/prov_mecklenburg.json`, `prov_thuringia.json`, `prov_brunswick.json`.

### fe4b-c4-allemagne-du-sud.md — FE4b — grappe C4 « Allemagne du Sud » (Souabe, Franconie, Tyrol) (2026-09-28)
- Branche `feat/fe4b-empire`, dans le worktree `game_project-fe4b`. Travail de données uniquement
- (pas de commit, pas de `cargo test`, pas de pipeline géo — laissés à l'orchestrateur).
- 8 nouvelles provinces créées ou reconfigurées, 7 nouveaux titres, 7 nouvelles factions jouables,

### fe4b-c5-pays-bas-imperiaux.md — FE4b — grappe C5 « Pays-Bas impériaux » (Namur, Liège, Utrecht, Lorraine) (2026-09-28)
- Branche `feat/fe4b-empire`, travail effectué dans `/Users/jean_hubert/dev/game_project-fe4b`.
- Aucun commit, aucun test, aucune régénération géo effectués (mandat de l'orchestrateur).
- Pas de nouvelle province : réaffectation de 4 provinces existantes (`prov_namur`, `prov_liege`,

### fe4b-empire.md — FE4b — Registre féodal du Saint-Empire et des Pays-Bas (1337) (2026-09-28)
- Branche `feat/fe4b-empire`, issue de `main` (a2672d2e). Recette F4a (voir
- `docs/wip/fe4a-france.md`, `docs/wip/fe4a-assets.md`).
- Cible indicative du mandat : ≈ +45 factions / ≈ +55 provinces. Contrainte du mandat :
- Réf. : commits a2672d2e

### fe4c-iles.md — FE4c — Registre féodal des îles Britanniques (1337) (2026-09-28)
- Branche `feat/fe4c-iles`, issue de `main` (a2672d2e). Recette : F4a (voir
- `docs/wip/fe4a-france.md`, `docs/wip/fe4a-assets.md`).
- 9 nouvelles factions jouables, 11 nouvelles provinces. Cible de l'orchestrateur ≈10/≈12 ;
- Réf. : commits a2672d2e, bdd997c1

### fe4d-iberie.md — FE4d — Registre féodal de l'Ibérie (1337) (2026-09-28)
- Branche `feat/fe4d-iberie`, issue de `main` (fba2e7ce). Recette F4a (`docs/wip/fe4a-france.md`,
- `docs/wip/fe4a-assets.md`). `CARGO_TARGET_DIR=core/target-fe4d`.
- 8 nouvelles factions jouables, ~10 nouvelles provinces, plusieurs titres « en titre » sans
- Réf. : commits fba2e7ce

### fe4e-italie.md — FE4e — Registre féodal de l'Italie (1337) (2026-09-28) [restes]
- Branche `feat/fe4e-italie`, issue de `main` (0cdafa9c, qui contient F0-F5).
- Titres, provinces, factions créés (données brutes, pipeline géo et colonies pas encore faits).
- 14 nouvelles provinces : Mantoue, Ferrare, Saluces, Pise, Sienne, Lucques (occupée par Vérone),
- Réf. : commits 0cdafa9c

### fe5-ia.md — FE5 — IA féodale (`feat/fe5-ia`, worktree `../game_project-fe5`) (2026-09-28)
- Spec FE § 4.2-4.4, 4.6, 5 ; plan section F5 ; ADR 0110. `CARGO_TARGET_DIR=core/target-fe5`.
- Données : `data/ai/feudal.json` + `data/schemas/ai_feudal.schema.json` (+ test pytest),
- `AiFeudal` (data-model, `GameData::ai_feudal`) ; `rank_strategies.county` dans `doctrines.json`.
- Réf. : ADR 0110 ; commits a9f7b409

### fe6-ui.md — FE6 — Interface de la féodalité (`feat/fe6-ui`, worktree `../game_project-fe6`) (2026-09-28) [restes]
- Spec FE § 6 (et § 4) ; plan section F6 ; ADR 0098. `CARGO_TARGET_DIR=core/target-fe6`.
- Cœur : `sim_campaign::feudal::view` (fiche, obligations, fil d'Ariane, carte féodale, états
- des vassaux, candidats à l'hommage) + tests `sim-campaign/tests/feudal_view.rs`.
- Réf. : ADR 0098
- Restes : Bretagne, Flandre, Navarre (départs recommandés de la spec) ne sont pas jouables dans les données

### fe7-portraits.md — FE7 — Portraits (état de travail) (2026-09-28)
- Branche `feat/fe7-portraits` (worktree `/Users/jean_hubert/dev/game_project-fe7`), issue de
- `main` a7db2d76. Plafond propre : 15 $ (section « Féodalité FE » de `docs/budget.md`, ADR 0098).
- Dry-run `uv run --project tools cent-ans assets portraits --dry-run` : **62 portraits
- Réf. : ADR 0063, 0098 ; commits a7db2d76, 56f9ef50, c8825f44

## Grappe `ib`

### ib1.md — IB1 — infobulles en sections (rendu § 2.1-2.3) — fichier de reprise (2026-09-28) [restes]
- Branche `feat/ib1-layout` (worktree agent). Spec : `docs/superpowers/specs/2026-09-28-ib-infobulles-design.md`
- § 2.1-2.3, ADR 0109, orchestration `docs/wip/ib.md`.
- 1. `game/tests/ib_shot.gd` (fenêtré, `--mode=avant|apres`) ; planche `docs/img/ib/avant/` (5 vues)
- Réf. : ADR 0109

### ib2.md — IB2 — migration mécanique (tables → données, ~120 infobulles brutes) — fichier de reprise (2026-09-28)
- Branche `feat/ib2-migrate` depuis `main` (05e711e1). Spec `docs/superpowers/specs/2026-09-28-ib-infobulles-design.md`
- § 2.4, ADR 0109, orchestration `docs/wip/ib.md`. IB1/IB3/IB4 déjà dans `main`.
- 1. Tables de `game/scripts/ui/rich_tooltip.gd` (`HUD_TEXTS`, `GAUGE_TEXTS`, `EFFECT_LABELS`,
- Réf. : ADR 0109 ; commits 05e711e1, 639b7e49

### ib3.md — IB3 — chaîne de bulles à Alt maintenu — fichier de reprise (2026-09-28)
- Branche `feat/ib3-chain` (depuis `main` c1775c8a, `main` a7d77308 fusionné). Spec § 3.1-3.2, ADR 0109.
- Fichiers : `game/scripts/codex/codex_bubbles.gd`, `game/tests/ib_chain_test.gd`,
- `game/scripts/ui/shortcut_sheet.gd` (ligne Alt). `tooltip_style.json` (bloc `chain`) en lecture.
- Réf. : ADR 0109 ; commits c1775c8a, a7d77308

### ib4.md — IB4 — liens `ib:`, bulles filles riches et de règle, placement — fichier de reprise (2026-09-28)
- Branche `feat/ib4-bubbles` (depuis `main` ee0a8816, `main` 0852a64f fusionné). Spec IB § 3.3-3.4,
- ADR 0109, orchestration `docs/wip/ib.md`. Fichiers : `codex_text.gd`, `codex_bubbles.gd`,
- `rich_tooltip.gd` (section IB4 en fin de fichier + `_icon_text`, `cost_text`, `_effects_block`,
- Réf. : ADR 0109 ; commits ee0a8816, 0852a64f

### ib5.md — IB5 — valeurs « avant → après » et état des prérequis (core + pont) — fichier de reprise (2026-09-28) [restes]
- Branche `feat/ib5-live` (worktree agent). Spec : `docs/superpowers/specs/2026-09-28-ib-infobulles-design.md`
- § 2.2 (effets) et § 4 (IB5), ADR 0109, orchestration `docs/wip/ib.md`. Cible cargo privée
- `core/target-ib5` (à supprimer à la fin).
- Réf. : ADR 0109

## Grappe `om`

### om-d1.md — OM-D1 — Registre Nord et Baltique (1337) (2026-09-28)
- Branche `feat/om-d1`. Périmètre : Danemark (interrègne), Suède-Norvège, Finlande, ordre Teutonique,
- Livonie, Poméranie. **État : TERMINÉ** (données ; géo et Rust non lancés, hors mandat).
- 33 provinces nouvelles, 12 factions nouvelles jouables, 15 titres nouveaux, 12 personnages, ~115 colonies.

### om-d2.md — OM-D2 — registre Europe centrale et orientale (1337) (2026-09-28)
- Branche `feat/om-d2`. Fichiers produits par un générateur jetable (hors dépôt) : ce sont les JSON qui font foi.
- 51 provinces nouvelles (`prov_*`), 15 factions jouables, 15 titres, 22 personnages, ≈ 110 colonies,
- 6 fichiers de noms (`names_pl/hu/lt/ro/sh/sk`), 12 maisons héraldiques, 15 fiches front-end.

### om-d3.md — OM-D3 — registre Rus' et Horde d'Or (1337) (2026-09-28)
- Branche `feat/om-d3`. Données seulement (pas de cargo, pas de géo).
- Données écrites (générateur jetable dans le scratchpad, données seules commitées) : 43 provinces, 129 colonies,
- 20 titres, 18 factions jouables, 12 personnages, 3 fichiers de noms (ru, tt, fu), 8 maisons, 18 fiches front-end,

### om-d4.md — OM-D4 — registre Balkans, Byzance, Égée (1337) (2026-09-28)
- Branche `feat/om-d4` (issue de `feat/om`). Brief : scratchpad `brief-common.md` + `brief-data.md`.
- 45 provinces (`prov_*`, régions `balkans`, `grece`, `egee`, `chypre`) et leurs colonies (3-5 chacune).
- 12 nouvelles factions jouables : `fac_byzantium`, `fac_epirus`, `fac_serbia`, `fac_bulgaria`, `fac_vidin` (vassal

### om-d5.md — OM-D5 — registre Anatolie, Caucase, Levant, Égypte (1337) (2026-09-28)
- Branche `feat/om-d5`. Données seulement (pas de cargo, pas de géo, pas de Godot). Générées par un script jetable
- (hors dépôt), contrôle d'intégrité perso OK (références, capitales possédées, colonies uniques, schémas, emprise EPSG:3035).
- 48 provinces (regions `anatolie`, `caucase`, `levant`, `egypte`, `mesopotamie`), 147 colonies, 23 factions jouables,

### om-d6.md — OM D6 — Maghreb et compléments méditerranéens (1337) (2026-09-28)
- Branche `feat/om-d6` (worktree `../gp-om-d6`), issue de `feat/om`. **État : TERMINÉ (données), reste l'intégration géo.**
- 6 factions jouables : `fac_marinids`, `fac_hafsids`, `fac_makki` (Gabès et Djerba), `fac_tripoli` (Banu Thabit),
- `fac_barqa` (tribus de Cyrénaïque), `fac_arborea` (Sardaigne, vassale d'Aragon). 6 titres, 6 personnages,

### om-om1.md — OM1 — monde rectangulaire (ADR 0115) (2026-09-28)
- Branche `feat/om-om1`, worktree `/Users/jean_hubert/dev/gp-om-om1`. Profil cargo `om1`.
- Plus aucune taille de monde codée en dur : `data/map/map.json` `size_px` [W, H] (multiples de 256).
- Carte actuelle 4096² : comportement identique.
- Réf. : ADR 0115

### om-om3.md — OM3 — terrains, climats et religions de l'Est (ADR 0116) (2026-09-28)
- `Terrain::Steppe`, `Terrain::Desert`, `Climate::Arid`, `Climate::Steppe` (schémas de règles étendus).
- Bataille : profils de décor `steppe`/`desert` (battle_decor.json), largeur de rivière
- (battle_water.json), relief de plaine, bois/villages rares ; auto-résolution (auto_resolve.json :
- Réf. : ADR 0116

### om.md — OM — carte Oural–Méditerranée (orchestration) (2026-09-29)
- Spec `docs/superpowers/specs/2026-09-28-oural-mediterranee-design.md`, plan
- `docs/superpowers/plans/2026-09-28-oural-mediterranee.md`, ADR 0115-0116.
- Worktree d'intégration `../game_project-om` (`feat/om`). Lancé le 2026-09-28 à 23 h, joueur absent
- Réf. : ADR 0115 ; commits 99b7e127, 5ca326bb, 9375189f

### om-c1.md — OM-C1 — complétude des données annexes (nouvelles provinces et factions) (2026-09-29)
- Branche `feat/om-c1` (depuis `feat/om`). État : TERMINÉ (données ; pas de cargo, Godot ni géo).
- `tools/tests/test_battle_speeches_schema.py` : liste `BATTLE_TERRAINS` complétée (steppe, desert).

### om-i1.md — OM I1 — intégration finale de la carte Oural–Méditerranée (2026-09-29) [restes]
- Branche `feat/om-om2`, worktree `../gp-om-om2`. Profil cargo `i1`.
- 1. Données D : provinces à 2 colonies, set_emba, colonies isolées sans port, autres incohérences.
- 2. Régénération géo complète (ordre de `om-om2.md`), commit dédié aux artefacts.
- Réf. : commits b9e8a069b

### om-om2.md — OM2 — chaîne géo sur l'emprise Oural–Méditerranée (2026-09-29)
- Branche `feat/om-om2`, worktree `../gp-om-om2`. ADR 0115 : EPSG:3035, 718,9765625 m/unité,
- 7168 × 6144, x 2 169 486 → 7 323 110, y 775 684 → 5 193 076 ; pixels existants : x identique,
- `tools/geo/raw/*` : liens symboliques vers les bruts du dépôt principal, sauf `kk10/`,
- Réf. : ADR 0115

### om-p2.md — OM P2 — portraits D4-D6 (branche feat/om-p2) (2026-09-29)
- 49 portraits de base (2,26 $ réel) + 42 variantes âgées `old` (1,94 $ réel) = 4,20 $ sur 4,50 $ ; cumul OM 8,05 $ sur 10 $.
- Entrées `aged_variants` ajoutées dans data/portraits/archetypes.json ; .import générés via godot --import.
- Rendu régional : uniquement via les données (titre, nom local, description) ; le style de base reste l'enluminure française (même recette que P1).

## Grappe `revue`

### revue-code.md — Revue de code globale (2026-09-26) (2026-09-28)
- État : TERMINÉ — fix/code-review fusionnée dans main (ff).
- Bilan : 55 constats traités sur 6 tranches → 50 corrigés, 3 en partie, 2 écartés (faux constats).
- Vérifications à l'intégration : cargo fmt/clippy -D warnings OK, cargo test workspace 856 ok / 0 échec,
- Réf. : ADR 0025 ; commits c0be4da7, 90343b88, b06cfc38

## Grappe `rs`

### rs-b-order.md — RS-B — économie et ordre public (cœur) (2026-09-28)
- Branche `feat/rs-b-order` (worktree d'agent). Orchestration : `docs/wip/restes.md`. ADR : 0100 (0098 pris par FE).
- Cible cargo privée : `core/target-rs-b` (à supprimer en fin de lot).
- 1. Constantes économiques de `economy.rs` → `data/rules/economy.json` (+ schéma, `EconomyRules`,
- Réf. : ADR 0100 ; commits 5c96ea30, 108709cc, ae5d94f1

### rs-c-diplo.md — RS-C — plafond d'opinion par motif et démolition (cœur, IA) (2026-09-28)
- Branche `feat/rs-c-diplo` (worktree d'agent, partie de main 48f1a5f8). Orchestration : `docs/wip/restes.md`.
- Cible cargo privée : `core/target-rs-c` (à supprimer en fin de lot). ADR : 0111 (0104-0106 réservés GA, 0107 pris).
- 1. Opinion « Mariage entre nos maisons » empilée sans plafond (+150), ambassades de héraut
- Réf. : ADR 0111 ; commits 48f1a5f8

### rs-d-trade.md — RS-D — Commerce et tests de campagne (fichier de reprise) (2026-09-28) [restes]
- Branche `feat/rs-d-trade` (depuis `main` c0be4da7). Orchestration : `docs/wip/restes.md`.
- Cible cargo privée : `CARGO_TARGET_DIR=core/target-rs-d` (à supprimer à la fin).
- Hors périmètre (RS-B en parallèle) : `economy.rs`, `settlements.rs`, `population.rs`, `medicine.rs`, `economy.json`.
- Réf. : commits c0be4da7, fbfca366

### rs-f-battle.md — RS-F — Bataille : incendie et sélecteur de formations (fichier de reprise) (2026-09-28)
- Lot F du chantier RS (`docs/wip/restes.md`). Branche `feat/rs-f-battle`. ADR réservé : 0099.
- Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-rs-f` (à supprimer à la fin).
- 1. Ordre « Incendier » dans l'UI (S2, `s1-s2-physique-incendies.md`).
- Réf. : ADR 0099

### rs-g-avignon.md — RS-G lot 1b — Avignon vers 1340 à l'échelle 1:1 (VH8) (2026-09-28)
- Branche `feat/rs-g-cities` (worktree partagé avec les autres villes VH8). Fichiers du lot :
- `data/landmarks_v2/avignon.json`, `tools/tests/test_landmarks_v2_avignon.py`, ce suivi.
- Format : `docs/landmarks-v2.md` ; exemple Rouen. Caches OSM hors dépôt :

### rs-g-bordeaux.md — RS-G lot 1a — Bordeaux v2 vers 1340 (VH8) (2026-09-28) [restes]
- Branche `feat/rs-g-cities`. Fichiers : `data/landmarks_v2/bordeaux.json`,
- `tools/tests/test_landmarks_v2_bordeaux.py`. Format : `docs/landmarks-v2.md`.
- 28/09 : squelette valide (origine sur la cathédrale Saint-André, OSM ; fleuve fin « Garonne »,

### rs-g-bruges.md — VH8 — Bruges vers 1340 à l'échelle 1:1 (format v2, ADR 0078) (2026-09-28)
- Branche `feat/rs-g-cities` (worktree partagé avec trois autres villes). Référence :
- `docs/landmarks-v2.md`, exemple Rouen. Fichiers du lot : `data/landmarks_v2/bruges.json`,
- `tools/tests/test_landmarks_v2_bruges.py`, ce suivi. Script d'auteur non versionné (scratchpad).
- Réf. : ADR 0078

### rs-g-calais.md — RS-G / VH8 — Calais vers 1340 à l'échelle 1:1 (format v2) (2026-09-28)
- Branche `feat/rs-g-cities` (worktree partagé avec d'autres villes). Fichiers : `data/landmarks_v2/calais.json`,
- `tools/tests/test_landmarks_v2_calais.py`, ce suivi. Caches OSM (non versionnés) :
- `tools/geo/raw/osm/calais_highways.json`, `calais_features.json`, `calais_features2.json`.

### rs-h-cleanup.md — RS-H — Nettoyage (28/09) (2026-09-28)
- Branche `feat/rs-h-cleanup` depuis `main`. Orchestration : `docs/wip/restes.md`.
- 1. **Mock / EventKind** (`game/scripts/sim/campaign_sim_mock.gd`) : l'événement fictif
- `"movement"` (armée qui campe, pure saveur, aucune règle) a été retiré de `_fake_events()` —

### rs-j-pavise.md — RS-J — Pavois face à une cible cachée (2026-09-28)
- Branche `feat/rs-j-pavise` (depuis `main`). Chantier RS (`docs/wip/restes.md`), point ouvert de
- RS-D (`docs/wip/revue-code.md`, n° 2 : les pavois attendent une cible cachée).
- `start_attack` (`src/sim/queue.rs`) : les pavois ne restent levés que si la cible est à portée

### rs-k-perf.md — RS-K — perf de la carte : TownLayer, `qt/collect`, NextHintController (+ effets de vie) (2026-09-28)
- Branche `feat/rs-k-perf` (worktree d'agent, depuis `main` 68050e4f). **Aucun changement Rust**
- (dylib copiée du dépôt principal), pas d'ADR (pas d'arbitrage d'architecture). Liens non
- versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal.
- Réf. : ADR 0051 ; commits 68050e4f

### rs-k2-perf.md — RS-K2 — perf carte (suite de RS-K) : colonies aux pas d'échelle, effets de vie (2026-09-28)
- Branche `feat/rs-k2-perf` (depuis `main` 5aae540f). **Aucun changement Rust** (dylib copiée), pas
- d'ADR (application d'ADR 0051 : étalement sur 1-2 images). Liens non versionnés
- `data/map/pyramid`, `tools/geo/raw` → dépôt principal.
- Réf. : ADR 0051 ; commits 5aae540f

### rs-l-alerts.md — RS-L — Détail du siège dans `get_provinces_snapshot` — fichier de reprise (2026-09-28) [restes]
- Branche `feat/rs-l-alerts`, depuis `main` (48f1a5f8). Suite de la note E (`docs/wip/restes.md`,
- `docs/wip/rs-e-map.md` point 2) : `game/scripts/ui/alerts.gd::collect` gardait un appel
- `get_province_state` par province assiégée pour lire le détail du siège (`attacker`, `supplies`,
- Réf. : commits 48f1a5f8

### rs-k3-perf.md — RS-K3 — perf carte (suite de RS-K2) : étiquettes, dé-encombrement, hameaux (2026-09-29)
- Branche `feat/rs-k3-perf` (depuis `main` 104f7a43). Aucun changement Rust visé (dylib copiée),
- pas d'ADR (application d'ADR 0051 : étalement sur 1-2 images). Liens non versionnés
- `data/map/pyramid`, `tools/geo/raw` → dépôt principal.
- Réf. : ADR 0051 ; commits 104f7a43, 5627d860

## Grappe `dv`

### dv1.md — DV1 — câblage des deux vues (2026-09-29)
- Branche `feat/dv1` (base `feat/dv`). Voir `docs/wip/dv.md`, ADR 0124.
- `StrategicView.weight_at` → `ZoomTiers.strategic_weight` ; `start/end_distance` retirés ;
- `overlay.draw_towns = true`.
- Réf. : ADR 0124

### dv2.md — DV2 — lieux sans pictogramme (écu + nom), HeraldryAtlas (2026-09-29)
- Branche `feat/dv2` (depuis `feat/dv`, fusionnée avec 666cb40fa). Spec DV, ADR 0124 (Écarts priment).
- `HeraldryAtlas` (`game/scripts/map/heraldry_atlas.gd`) : atlas des écus extrait de
- `SettlementMarkers` ; retient aussi les factions sans armoiries (plus de recomposition à
- Réf. : ADR 0124 ; commits 666cb40fa

### dv.md — DV — deux vues de la carte de campagne (2026-09-30)
- Spec : `docs/superpowers/specs/2026-09-29-dv-deux-vues-campagne-design.md`. ADR 0124.
- Plan : `~/.claude/plans/docs-superpowers-specs-2026-09-29-dv-deu-delightful-snail.md`.
- Worktree `../game_project-dv`, branche `feat/dv`. Coût cloud : 0 $.
- Réf. : ADR 0124

## Grappe `fk`

### fk1-core.md — FK1 — cœur Rust de la carte vivante (2026-09-29)
- Branche `feat/fk1-core`. Spec `docs/design/2026-09-29-carte-vivante-folk.md` §§ 2.1, 6, 7 ;
- ADR 0122 ; note chantier `docs/wip/fk.md` (non modifiée ici pour éviter les conflits de vague).
- `data-model` : `Event.map_scene: Option<SceneKind>`, `Event.presentation: Option<EventPresentation>`,
- Réf. : ADR 0122

### fk2-assets.md — FK2 — assets Blender de la carte vivante (2026-09-29)
- Branche `feat/fk2-assets`. Spec : `docs/design/2026-09-29-carte-vivante-folk.md` § 2.3, § 3.
- `tools/blender_scripts/folk_props.py` → `game/assets/models/folk/*.glb` + `manifest.json`
- (15 modèles, mètres, +X devant, créneaux en repère Godot).

### fk3-folk.md — FK3 — réservoir de figurines, vie ordinaire, marchands (branche `feat/fk3-folk`) (2026-09-29) [restes]
- Spec : `docs/design/2026-09-29-carte-vivante-folk.md` §§ 2.2, 3.1, 3.2, 5-7. Suivi général : `docs/wip/fk.md`.
- `life_folk/folk_models.gd` (`FolkModels`) : table rôles → figurines skinnées (candidats, FK2 en
- tête : `civilian_0..3`), activités → clips (`plough`, `scythe`, `carry` de FK2 pris dès qu'ils
- Restes : Trajets rectilignes en boucle (pas de suivi de polyligne en shader) : une charrette parcourt un / Clés lues dans `map_scenes.json` (premier niveau, `folk` ou `densities`) : `pool_cap`,

### fk4-scenes.md — FK4 — scènes de province (rendu GDScript) (2026-09-29) [restes]
- Branche `feat/fk4-scenes` (base `integration/fk`, 299233c8c). Spec
- `docs/design/2026-09-29-carte-vivante-folk.md` §§ 2.2, 3.3, 5-7 ; suivi `docs/wip/fk.md`.
- Réglages alignés : `data/rules/map_scenes.json` = source unique (schéma, `MapSceneRules`,
- Réf. : commits 299233c8c

### fk5-incidents.md — FK5a — incidents sur la carte (rendu + UI) (2026-09-29) [restes]
- Branche `feat/fk5-incidents` (base `integration/fk` 299233c8c). Spec
- `docs/design/2026-09-29-carte-vivante-folk.md` §§ 2.1.2, 3.4, 6, 7 ; ADR 0122.
- Cœur : `CampaignState::debug_offer_decision` (mise en scène, `chronicle.rs`) + pont
- Réf. : ADR 0122 ; commits 299233c8c
- Restes : Pictogramme unique (`hud_chronicle_decision`) pour tous les incidents : un pictogramme par / L'avis d'expiration repère l'entrée de chronique par la marque « (délai écoulé) » du cœur / `--no-folk` coupe aussi les sceaux (repli sur la fenêtre de début de tour).

### fk5b-events.md — FK5b — 15 événements du quotidien (2026-09-29)
- Branche `feat/fk5-events`. 15 événements `random` de province dans `data/events/`.
- 15 événements (2-3 options, sources, map_scene quand un type colle)
- pytest 1274 verts, cargo test vert

## Grappe `nt`

### nt2-bataille-perso.md — NT2 — Bataille personnalisée (2026-09-29)
- Branche `feat/nt2-custom-battle`. Spec : `docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md` (ligne NT2).
- Cœur : `sim-battle/src/custom.rs` (configuration, roster, budget, validation, `BattleSetup`),
- météo forcée via `ReplayStart.weather` ; règles `data/rules/custom_battle.json` (+ schéma).

### nt.md — NT — nuit niveau TWW3 (2026-09-30)
- Spec : `docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md`. ADR : 0126 (types de places),
- 0127 (missions), 0128 (plafond et engins). Coût cloud : 0 $.
- NT1 sièges variés (château, bourg fortifié, cité), choisissables aussi en bataille personnalisée
- Réf. : ADR 0096, 0128, 0129 ; commits 53964fcdf, 063999ef0

### nt1-sieges.md — NT1 — Sièges variés (château, bourg fortifié, cité) (2026-09-30) [restes]
- Spec : `docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md` (ligne NT1). ADR 0126.
- Branche : `feat/nt1-siege-layouts`. Cargo : `CARGO_TARGET_DIR=core/target-nt1`.
- Données : bloc `places` de `data/rules/siege_town.json` + schéma (type par genre de localité et
- Réf. : ADR 0126
- Restes : Bourg moins dense que la cité (≈ 40 îlots) : à juger visuellement, réglable dans `places.borough`. / Donjon rendu en tour ronde agrandie (pas de maquette de donjon carré dans le kit). / Équilibre d'un assaut de château (petite enceinte, garnison serrée) à surveiller en partie pilote.

### nt10.md — NT10 — fondu des rôles, imposteurs (casques, ombre, sang), herbe hors champ (2026-09-30)
- Branche `feat/nt10-anim-crowd` (worktree agent). Orchestration : `docs/wip/nt.md` (vague 4).
- Références : ADR 0129 (fondu NT7), `docs/wip/nt7-anim.md`, `docs/wip/bv3-finitions-bataille.md`.
- 1. Fondu des clips de rôle (mode CUSTOM) : porte-étendards, musiciens (`BattleStandards`) et
- Réf. : ADR 0129

### nt11.md — NT11 — restes NT (camp tenu, bataille personnalisée, donjon) (2026-09-30) [restes]
- Branche `feat/nt11-leftovers`. Cargo : target privé `core/target-nt11`, `CARGO_PROFILE_DEV_DEBUG=0`.
- 1. Camp tenu (cœur `sim/hold.rs`, `BattleSim::set_hold/holds`, `ReplayAction::SetHold`,
- pont `set_hold/get_hold`). Tant qu'il est actif : l'IA du camp ne garde que les ordres qui
- Restes : Escalier et palier hors de l'emprise de la simulation (rendu seul, ~3,4 m devant la façade). / Pas de capture faite (consigne) : escalier et terrasse à juger à l'œil (`nt1_siege_shot.gd`). / Dans le prologue, l'ennemi peut toujours se débander sous la charge et les flèches.

### nt12-mocap.md — NT12 — essai de capture de mouvement gratuite (2026-09-30)
- Branche `feat/nt12-mocap-trial` (worktree agent). Orchestration : `docs/wip/nt.md` (§ Pour le
- joueur, point 1). Sources, licences, défauts, verdict : `docs/research/mocap-gratuite.md`.
- Téléchargées (curl, sans compte) : CMU Graphics Lab (ASF/AMC 120 i/s) dans

### nt13-video-mocap.md — NT13 — vidéo vers animation (mocap maison) (2026-09-30)
- Branche `feat/nt13-video-mocap`. Orchestration : `docs/wip/nt.md`. Prédécesseur : NT12
- (`docs/wip/nt12-mocap.md`, reciblage CMU, `--mocap-trial`).
- Vidéos du joueur (hors dépôt) : `~/dev/cent-ans-mocap-src/video/IMG_6455..6458.MOV` (4K

### nt14-video-set2.md — NT14 — vidéo vers animation, second tournage (2026-09-30)
- Branche `feat/nt14-video-set2`. Orchestration : `docs/wip/nt.md`. Prédécesseurs : NT13
- (`docs/wip/nt13-video-mocap.md`), NT12 (`docs/wip/nt12-mocap.md`).
- Vidéos (hors dépôt) : `~/dev/cent-ans-mocap-src/video/IMG_6459..6461.MOV` (1080×1920 après

### nt3-missions.md — NT3 — missions de campagne (2026-09-30)
- Branche `feat/nt3-missions`. Spec : `docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md` (NT3).
- ADR : `docs/decisions/0127-missions-de-campagne.md`.
- Données `data/missions.json` + schéma `data/schemas/missions.schema.json`, chargement `GameData::mission_rules`
- Réf. : ADR 0127 ; commits df36171e4

### nt5-plafond-engins.md — NT5 — N6 plafond d'unités + N7 engins de siège construits (2026-09-30)
- Branche `feat/nt5-cap-engines` (worktree agent). État : **terminé**, à fusionner. Spec : `docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md` (ligne NT5).
- ADR : `docs/decisions/0128-plafond-et-engins-de-siege.md`.
- N6 : `data/rules/armies.json` (`max_units` 20, schéma `army_rules.schema.json`), `GameData::army_rules`.
- Réf. : ADR 0128 ; commits f36689196, 35667be6e

### nt6ab.md — NT6ab petites suites (2026-09-30)
- Branche `feat/nt6ab-leftovers`. Termine.
- 1 discours adverse : `BattleScene._start_enemy_speech`, `BattleSpeech.skipped` (passer = pas de discours adverse), meme reglage `--no-speech`, voix via `VoiceLines` si fichiers existants.
- 2 indicateur vise : `BattleSiege._update_target_marks` (champ `target` des unites).

### nt6cd.md — NT6 c et d — petites suites (IA au repos, réaffectation des touches) (2026-09-30)
- Branche `feat/nt6cd-leftovers`. État : TERMINÉ.
- Données : `postures.rest` de `data/ai/grid.json` (`enabled`, `below_percent` 60, `until_percent` 85,
- `watch_radius_km` 60), schéma et `AiRest` (data-model).

### nt7-anim.md — NT7 — fondus entre clips de mêlée et clips de rôle (porte-étendard, musicien, servants) (2026-09-30)
- Branche `feat/nt7-anim-blend` (worktree agent). Orchestration : `docs/wip/nt.md`.
- ADR : `docs/decisions/0129-fondu-cycle-et-clips-de-role.md` ; ADR 0096 « Limites » mis à jour.
- Figurines : MultiMesh + texture d'os cuite (`battle_soldier_skinned.gdshader`), pas
- Réf. : ADR 0096, 0129

### nt8-chateau.md — NT8 — Rendu du château de siège (2026-09-30)
- Chantier NT (`docs/wip/nt.md`), suite de NT1 (ADR 0126, `docs/wip/nt1-sieges.md`).
- Branche : `feat/nt8-castle-look`. Cargo : target privé `core/target-nt8`, `profile.dev.debug=0`.
- Capture `docs/audit/captures/nt/nt1_castle.png` : tours du château énormes, donjon = tour ronde
- Réf. : ADR 0126

### nt9-equilibre.md — NT9 — équilibre après N6/N7 (2026-09-30)
- Branche `feat/nt9-balance` (worktree agent). Lot NT9 de `docs/wip/nt.md`. ADR 0128 (révision NT9),
- ADR 0127 (sorties). État : **terminé**, à fusionner.
- Sonde : `BATTLE_TRACE=1` (`century_probe`) imprime chaque ligne « bataille FR/EN » comptée ;
- Réf. : ADR 0127, 0128 ; commits f36689196, 35f40385d

## Grappe `omr`

### omr-r3.md — OMR R3 — équilibre Est (commise, banqueroutes, révoltes) (2026-09-29)
- Worktree `../gp-omr-r3`, branche `feat/omr-r3`. Cible cargo partagée `core/target`, profil `r3`.
- `CARGO_TARGET_DIR=<main>/core/target cargo build -p ai --example century_probe --config 'profile.r3.inherits="release"' --config 'profile.r3.debug=0' --profile r3`
- puis `<main>/core/target/r3/examples/century_probe <tours> <graines…>` (`VERBOSE=1` : banqueroutes par faction).
- Réf. : ADR 0114, 0117 ; commits 5d64d2bd

### omr-r4.md — OMR R4 — relecture historienne de l'Est (printemps 1337) (2026-09-29) [restes]
- Branche `feat/omr-r4`, worktree `../gp-omr-r4`. Ne touche ni revenus, ni garnisons, ni bâtiments
- (R3) : les colonies remplacées gardent le profil de bâtiments de celles qu'elles remplacent.
- 1. Objectifs ajoutés par I1 (30 objectifs nouveaux sur 31 titres ; Galicie : retrait de
- Restes : Kholmogory (1355), Kotelnitch, Vychni Volotchek (cité au XVe s.) : gardés comme localités / Sozopolis : bulgare ou byzantine en 1337 (laissée avec Anchialos, bulgare). / Mourom rattachée à Souzdal (principauté de Mourom autonome), Tchernigov à Briansk.

### omr-r5.md — OMR R5 — unités propres à l'Est (1337) (2026-09-29)
- Branche `feat/omr-r5`, worktree `../gp-omr-r5`. Plan d'ensemble : `docs/wip/omr.md`.
- Recrutement : règles existantes seules (culture de province ET faction, époque) ; pas de règle
- nouvelle dans `core/`, donc pas d'ADR.

### omr-r6.md — OMR R6 — musique orthodoxe/islamique + portraits (branche feat/omr-r6) (2026-09-29)
- Partie A : `era_music.py` étendu (champ `max_seconds`, pause anti-limite de débit de l'API
- Commons, correction du fichier temporaire ffmpeg `.part.mp3`). Contextes `campaign_orthodox`
- (5 pistes : 2 hymnes byzantins, 3 chants znamenny, CC0) et `campaign_islamic` (5 pistes :

### omr-r7.md — OMR R7 — relief fin à l'Est et au Sud (paliers 1, E1-E2) (2026-09-29)
- Lot R7 du chantier OMR (`docs/wip/omr.md`). Worktree `../gp-omr-r7`, branche `feat/omr-r7`.
- Objectif : étendre la pyramide de relief fin (aujourd'hui Ouest seulement, cadre 16 × 16 décalé
- `root_origin_tiles` [0, 5]) à tout le monde OM (28 × 24 tuiles racines), pour qu'un zoom sur
- Réf. : ADR 0077, 0121

## Grappe `cr`

### cr.md — CR1 — artefacts des gros plans de bataille (2026-09-30)
- État : CR1 terminé (6 artefacts corrigés), grille avant/après `docs/img/cr/cr1_ab.jpg` (captures brutes `~/dev/cent-ans-raw/cr1/`).
- Prochaine étape : jugement du joueur ; points ouverts plus bas. Références : `~/dev/cent-ans-raw/cav/c1_mounted.jpg`, `c2_close4.jpg`.
- Prochaine étape : bissection par options `--no-*`, identification du code fautif pour les 6 artefacts
- Réf. : commits 3dfbabb01, 7abcf6bd4, 0c02393d0

## Grappe `ga`

### ga3.md — GA3 — Sondes image-vers-3D (fal.ai TRELLIS) (2026-09-30)
- Branche `feat/ga3`, worktree `../game_project-ga3`. Clé `FAL_KEY` dans l'environnement (vérifiée 30/09).
- Plan d'origine : `docs/wip/ga.md` §GA3. Budget GA3 ≤ 8 $ (sondes < 1 $). Dépenses → `docs/budget.md`.
- Ne pas toucher `feat/sr` (autre session, figurines semi-réalistes).
- Réf. : ADR 0140 ; commits fbfd95815

### ga3-l2.md — GA3-L2 — végétation réaliste de la carte de campagne (2026-09-30) [restes]
- Worktree `game_project-ga3`, branche `feat/ga3`. Spec : `docs/wip/ga3.md` (S5, « Lots de production »).
- Squelette : `game/scripts/map/ga3_vegetation.gd` (option `--no-ga3-veg`, `--no-ga3-near`, chemins).
- Génération fal (0,474 $) : `tools/blender_scripts/ga3_vegetation_l2.py fal` ; brutes `~/dev/cent-ans-raw/ga3/l2/`.
- Réf. : ADR 0140

### ga3-l3.md — GA3-L3 — Figurines de bataille générées (L3a longbowman, L3b 4 unités à pied) (2026-09-30)
- Branche `feat/ga3`, worktree `../game_project-ga3`. Budget L3a ≤ 2 $ : dépensé 0,48 $
- (`docs/budget.md`). Brutes : `~/dev/cent-ans-raw/ga3/l3/longbowman/` (planche, découpes,
- `multi_front_back.glb`, `trellis.glb`, `trellis2.glb`, rendus `renders_<modèle>/`, `.blend`).
- Réf. : ADR 0140

### ga3-l4.md — GA3-L4 — Figurines générées : défauts + recettes restantes (2026-09-30)
- Branche `feat/ga3-l4` (worktree agent). Budget L4 ≤ 3 $ (cumul GA3 de départ 6,55 $).
- Brutes : `~/dev/cent-ans-raw/ga3/l4/`. Contexte : `docs/wip/ga3.md` (L3a-c), `docs/wip/ga3-l3.md`.
- Lis ce fichier et `git log --oneline -15`, continue à la première case non cochée.
- Réf. : ADR 0140

### ga3-l5.md — GA3-L5 — trébuchet et bélier générés branchés sur les engins animés (2026-09-30)
- Branche `feat/ga3-l5`. Contexte : `docs/wip/ga3.md` (L1, L1b), ADR 0140 § « Engins de siège (L5) ».
- Pas de régénération (0 $) : les LOD GA3 existants (`props_ga/ga3_{trebuchet,ram}_lod{0,2}.glb`,
- déjà UV et texturés) sont **découpés par régions** (centroïdes des faces) dans Blender, chaque
- Réf. : ADR 0140

## Grappe `ia`

### ia-quality-probe.md — Sonde de qualité de l'IA de campagne (`ia_quality_probe`) (2026-09-30) [restes]
- Branche `feat/ia`. Exemple `core/crates/ai/examples/ia_quality_probe.rs` : mesure la qualité du
- jeu de l'IA (prises manquées, pertes sans secours, armées oisives, batailles suicidaires,
- fragmentation, sièges, trésor, coût par tour), IA contre IA depuis 1337. Mesure seule : aucune

### ia-nuit.md — IA — nuit du 2026-09-30 : IA performante en campagne et en bataille (2026-10-01)
- Branche `feat/ia` (worktree `../gp-ia`, depuis `main` 6a0d960c9). Mandat du joueur : « tu
- travailles sur l'IA du jeu et tu t'assures qu'elle soit performante en campagne ou en bataille »,
- « Performante » = les deux sens : l'IA joue bien (bataille : bat une IA naïve à forces égales,
- Réf. : ADR 0148 ; commits 6a0d960c9, 92ec4dc18

## Grappe `nb`

### nb.md — NB — Nano Banana 2 : ancre de style et interface (orchestration) (2026-09-30)
- Spec : `docs/superpowers/specs/2026-09-30-nb-nano-banana-interface-design.md` (0d0498bde, approuvée).
- Branche `feat/nb`, worktree `../game_project-nb`. Fusion `--ff-only` dans `main` après jugement.
- Budget : plafond 10 $, section « Nano Banana NB » de `docs/budget.md` (cap passé explicitement
- Réf. : ADR 0135 ; commits 0d0498bde

## Grappe `sl`

### sl1-routes-maritimes.md — Lot SL1 — Routes maritimes (2026-09-30) [restes]
- Branche `feat/sea-lanes`. ADR : `docs/decisions/0139-routes-maritimes.md`.
- Demande : « il manque les routes maritimes dans le jeu (visuellement et avec des mécaniques de jeu) ».
- Constat : le graphe des colonies n'a que de courts passages entre provinces voisines (Douvres–Wissant…) ;
- Réf. : ADR 0139

## Grappe `ss`

### ss1-colormap.md — SS1 — carte de couleur du sol (`cent-ans geo colormap`) (2026-09-30)
- Branche `feat/ss-colormap` (worktree `../gp-ss-cm`), fusion par l'orchestrateur dans `feat/ss`.
- Module `tools/cent_ans_tools/geo/colormap.py` : 5 couches (base régionale, mosaïque Voronoï + lanières + bocage + vigne, auréoles de villes, routes, lacs), peinture par bandes (hachage sur coordonnées monde : indépendant des band…
- Style `data/map/colormap_style.yaml`, schéma `data/schemas/colormap_style.schema.json`.

## Grappe `tf`

### tf.md — TF — Colombage TimberFrame + toits du château Kenney (lot L4 de realisme-suite) (2026-09-30) [restes]
- Branche `feat/tf`, worktree `/Users/jean_hubert/dev/game_project-tf`. Décisions : ADR 0105 §TF.
- Atlas : `TimberFrame` couche 14 (fin de tableau), tableaux d'albédo/normales à 15 tranches
- (`build_textures.py`), shaders `building_atlas`/`town_building` : bloc uni
- Réf. : ADR 0105
- Restes : Atlas `Building` plein : une nouvelle matière demandera 32 tranches. / Maquettes de colonies CV1 et monuments non réexportés (panneaux `Plaster`). / Toits du Midi : seules les variantes `southern` ont des tuiles canal ; longères, granges et

## Grappe `vn`

### vn-ui-720.md — VN — défauts d'interface en 1280x720 (2026-09-30)
- Lot 1 (menu, tutoriel vs modale, province vs mini-carte, bandeau) : fait, `game/tests/vn_ui_720_test.gd`.
- Lot 2 : chronique (largeur de contenu bornée à la zone SIDE_PANEL), fiche de ville (lignes vides masquées), journal en tête de pile TOASTS et masqué sous objectifs/cour/techs, tableau du budget (libellés à la ligne), cartouche de…
- q6_ui_test et fe_ui_test échouent aussi sur main (pas causés par VN).

### vn.md — VN — nuit visuelle (2026-09-30) (2026-10-01) [restes]
- Mandat : amélioration visuelle libre, autonomie toute la nuit, 100 captures, fal.ai possible
- (pas OpenRouter). Branche `feat/vn`, worktree `../gp-vn`. La carte de campagne (biomes, forêts,
- rochers) appartient à HB (`feat/hb`) : VN n'y touche pas.
- Réf. : commits 7e68b23e3, 147affacd
- Restes : 147 factions sans miniature d'encyclopédie (~12 $ en fal.ai) + bld_collegiate_church : non fait / Tours de siège énormes (rayon 5+fortif m, cœur `siege_layouts.rs`) : règle du cœur, session / `q6_ui_test` (SimFacade introuvable avec --script) et `fe_ui_test` échouent aussi sur main.

## Grappe `vt`

### vt-b-far-builder.md — VT-B — TownFarBuilder (maillage lointain des villes) (2026-09-30)
- État : fait. `game/scripts/map/town_far_builder.gd` (F1, F2, v2, `append`, `mesh_arrays`,
- `triangle_count`) + `game/tests/tf_far_mesh_test.gd` (passe).
- Mesures (2 134 villes, un fil) : F1 294 tri/ville (max 706, Milan), F2 50,5 (max 56) ;

### vt-d.md — VT-D — retraits des maquettes dans `settlement_layer.gd` (2026-09-30) [restes]
- Lot D du chantier VT (ADR 0138, `docs/wip/vt-plan.md`, sections Retraits et Recâblage).
- Retrait des maquettes (colonies, L1), SZ4b, DC4/DC6c, ombres.
- Étiquettes au sol + décalage px au-dessus de l'emprise.
- Réf. : ADR 0138

### vt-plan.md — VT — plan technique (rapport de l'agent de planification, 30/09) (2026-09-30)
- 2 147 colonies (`data/settlements/prov_*`), 2 134 avec emprise dans `data/map/towns_1340.json` (438 cités, 699 villes, 418 villages, 335 châteaux, 244 abbayes) + 8 villes v2. Le « 570 » des commentaires est périmé.
- `TownPlan` coûte 0,5-2 s/ville : le lointain ne passe pas par lui.
- `towns_1340.json` a déjà : `radii` (32 relèvements), `faubourgs`, `walls`, `monuments` (`at`, `size_m`), `river`/`bridge`, `z_m`. Il manque des hauteurs fines sur l'emprise ; `towns.py:590` échantillonne déjà la pyramide.
- Réf. : ADR 0138

### vt2.md — VT2 — moulins, fumées et figurants FK à l'échelle 1:1 (2026-09-30)
- Suite de VT (ADR 0138, `docs/wip/vt.md`). Demande validée par le joueur : moulins, panaches de
- cheminée et figurants FK (gens, bêtes, charrettes ; ADR 0122) à l'échelle réelle à toute distance
- de la vue 3D. Incendies et arbres restent grossis (non touchés). Worktree `../game_project-vt2`,
- Réf. : ADR 0122, 0138

### vt3.md — VT3 — arbres de la carte à l'échelle 1:1 (2026-09-30)
- Suite de VT/VT2 (ADR 0138). Demande validée par le joueur : les arbres de la carte de campagne à
- l'échelle réelle à toute distance de la vue 3D ; au-delà de la portée où ils font ≈ 1 px, la forêt
- est portée par le terrain (canopée). Les incendies restent exagérés. Worktree
- Réf. : ADR 0138

## Grappe `vx`

### vx-voix-criees.md — VX — voix de combat criées (2026-09-30)
- Retour joueur (30/09) : « la voix était un peu molle en combat ; Montjoie, Saint-Denis ».
- gpt-audio-mini (VO1) lit les cris à hauteur de parole : « Montjoie ! Saint-Denis ! » à
- ~130 Hz médians, comme « Monseigneur ? » calme (114 Hz). Un cri d'homme monte à 200-350 Hz.
- Réf. : ADR 0145

## Grappe `hv`

### hv-verification-historique.md — WIP — HV : vérification historique (nuit du 30 septembre 2026) (2026-10-01)
- **État : terminé, fusionné dans main** (5eeb5f7b4 puis 39de6fd1e, d9e206429). Synthèse :
- `docs/histoire/audit-2026-09-30.md` ; rapports par lot `docs/histoire/audit-2026-09-30-hv1.md` à `-hv10.md`.
- Intégration : générateur d'écus (croix pisane, fleurdelisée, griffon, échiqueté), écus/bannières
- Réf. : commits 5eeb5f7b4, 39de6fd1e, d9e206429

