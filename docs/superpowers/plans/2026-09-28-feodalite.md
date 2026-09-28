# Plan : FE — féodalité et petites factions jouables

Spec : `docs/superpowers/specs/2026-09-28-feodalite-design.md` (validée le 2026-09-28).
Suivi : `docs/wip/fe.md` (orchestration) et `docs/wip/fe<N>-<objet>.md` (un par lot).
ADR : **0098** (titres au-dessus des factions). Budget : section « FE » de `docs/budget.md`, plafond propre 15 $ (portraits F7 seulement).

Organisation :
- F0 est fait par l'orchestrateur dans `main`. Il crée ensuite `feat/fe`.
- Chaque lot est confié à un agent en worktree sur `feat/fe<N>-...`, branche issue de `feat/fe`. L'orchestrateur fusionne dans `feat/fe` (fusion dans un worktree dédié), puis fait un ff vers `main` en fin de vague. Les agents ne touchent jamais à `main`.
- Au plus 6 agents par vague. `cent-ans-dev` pour les règles et l'IA, `cent-ans-mech` (Sonnet) pour les données, le câblage et les tests.
- Chaque worktree a son propre `CARGO_TARGET_DIR`. Commits `wip:` toutes les 15 min avec des chemins explicites. Jamais `git stash`.
- Rust : `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` avant chaque commit non-`wip`. Python : `uvx ruff check --fix`, `uvx ruff format`.
- Les agents n'utilisent pas les outils de capture. Les captures passent par les scripts `*_shot.gd`, lus par l'orchestrateur (3 au plus par lot).

Points d'ancrage existants (à réutiliser, pas à dupliquer) :
- Vassalité actuelle : `sim-campaign/src/diplomacy.rs`.
  - Constantes `VASSAL_TRIBUTE_PERCENT`, `REBELLION_LOYALTY`, `CALL_TO_ARMS_LOYALTY`, `REBELLION_PERMILLE`, `VASSALAGE_POWER_RATIO` : codées en dur, elles partent dans `data/rules/feudal.json`.
  - Fonctions `make_vassal`, `release_vassal`, `cut_vassal_tie`, `rally_vassals`.
  - Champ `suzerain` de l'état de faction (`state.rs:586`).
- Succession : `characters.rs::succeed` et `resolve_faction_deaths`, `dynasty.rs::pick_heir_by_law`, mariages dans `dynasty.rs`.
- Données : `data-model/src/entities/province.rs` (champs `overlord` et `holder`, à supprimer), validation dans `data-model/src/load.rs:725`.
- Version de sauvegarde : `state.rs` (commentaire de version l. 36).

---

## F0 — Fondations (orchestrateur, `main`)

1. **ADR** `docs/decisions/0098-titres-au-dessus-des-factions.md`. Contexte : départ trop grand, frontières 1337 exactes. Décision : titres *de jure* ; factions détentrices ; déductions jamais stockées ; 3 niveaux ; maxime. Conséquences : `overlord`/`holder` supprimés, sauvegardes invalidées.
2. **Schéma** `data/schemas/title.schema.json` (spec § 3.1). Ajout de `primary_title` au schéma de faction. Retrait d'`overlord` et `holder` du schéma de province.
3. **Modèle** :
   - `data-model/src/entities/title.rs` : `TitleId`, `TitleRank`, `Title`, `Objective` (types de la spec § 4.8), chargés dans `GameData.titles` ;
   - validation des invariants § 3.1 dans `load.rs`.
4. **Règles** : `data/rules/feudal.json` avec les constantes actuelles de `diplomacy.rs` (mêmes valeurs), plus `disloyal_threshold`, `felony_window_turns`, `independence_turns`, `ascension_turns` et les poids de loyauté. Les constantes deviennent des lectures de ces données.
5. **Migration** : `tools/cent_ans_tools/feudal_migrate.py`. À provinces constantes, il génère `data/titles/*.json` depuis `owner`/`overlord`/`holder` actuels : un royaume par souverain, un duché ou comté par vassal actuel et par province détenue par un personnage. Il retire ensuite les deux champs. Tests pytest : invariants, aucune province orpheline.
6. **Squelette cœur** `sim-campaign/src/feudal.rs` :
   - API publique vide : `liege_of(faction)`, `province_lieges(province)`, `direct_vassals(faction)`, `feudal_tree(faction)`, `war_escalation_preview(attacker, target)`, `open_felony`, `declare_commise`, `transfer_title`, `evaluate_objectives` ;
   - `sim-campaign/tests/feudal_*.rs` désactivés (`#[ignore]`).
7. **Sauvegarde** : version de format incrémentée ; message « sauvegarde d'une version antérieure » côté pont.
8. `docs/wip/fe.md`, section FE de `docs/budget.md` (0 $). Tous les tests existants passent (le comportement est inchangé à données migrées). Commit `FE0: titles skeleton, migration, ADR 0098`. Puis `git branch feat/fe main`.

## Vague 1 (4 agents)

### F1 — Déductions, obligations, loyauté (`feat/fe1-deductions`, `cent-ans-dev`)
- Implémenter les déductions (spec § 3.3) dans `feudal.rs`. Le champ `suzerain` de `state.rs` devient une vue dérivée des titres ; la source unique est la détention des titres.
- Brancher `diplomacy.rs` sur `feudal.rs` : tribut au suzerain **direct** seulement, ost par `rally_vassals` limité aux vassaux directs (maxime), loyauté selon les facteurs de la spec § 4.2 avec les poids de `feudal.json`.
- `Proposal::Vassalage` devient « prêter hommage » : transfert du `de_jure_liege` effectif du titre principal.
- Tests `feudal_deductions.rs` : double allégeance Guyenne, maxime (le roi ne lève pas le comte du duc), tribut, seuils de loyauté.

### F2 — Escalade et guerre privée (`feat/fe2-escalade`, `cent-ans-dev`)
- Dans `declare_war` : appel du suzerain direct de la cible. Décision (IA : score provisoire simple, remplacé en F5 ; joueur : événement). Intervention en cascade ; dérobade = prestige et loyauté (spec § 4.3).
- Guerre privée : si l'attaquant et la cible ont le même suzerain, celui-ci est appelé à arbitrer (paix imposée, parti pris, laisser faire).
- `war_escalation_preview` : chaîne + estimation `likely | uncertain | unlikely` + raison principale.
- Tests `feudal_escalation.rs` : chaîne comte → duc → roi, dérobade, arrêt de la cascade, guerre privée.

### F3 — Transferts de titres (`feat/fe3-titres`, `cent-ans-dev`)
- `transfer_title` : fusion (union personnelle), création d'une faction quand un titre est concédé à un personnage sans faction, disparition d'une faction sans titre, indépendance des vassaux quand le titre supérieur est vacant (§ 4.7).
- Commise : ouverture sur refus d'ost, alliance avec l'ennemi ou révolte ; casus belli limité au félon ; titre au suzerain à la paix.
- Héritage : `characters.rs::succeed` transmet chaque titre selon `pick_heir_by_law` ; déshérence ; plusieurs prétendants = guerre de succession arbitrée.
- Conquête : exigence de titre dans le traité de paix (`negotiation.rs`) ; usurpation ou concession selon le rang.
- Objectifs : `evaluate_objectives` à chaque tour ; conditions de victoire générique (§ 4.8) dans le module de victoire existant (`m10_victory`).
- Tests `feudal_titles.rs` : commise Guyenne, Bretagne 1341 (deux prétendants), Bourgogne 1361 (partage duché/comté), union personnelle, déshérence, victoire par indépendance.

### F4a — Registre France (`feat/fe4a-france`, `cent-ans-mech`)
- Subdiviser les provinces françaises pour les fiefs manquants (Alençon, Évreux, Albret, Charolais, Penthièvre, Bourbon, Blois, Foix-Béarn, Armagnac, Valois…). Nouvelles graines dans `data/provinces/`, `cent-ans geo provinces`, réaffectation des colonies.
- Titres, factions, souverains et héritiers (personnages), objectifs historiques, sources.
- Relecture historienne (agent dédié, rapport dans `docs/wip/fe4a-france.md`).
- Tests : pytest invariants ; `cargo test` (chargement des données).

## Vague 2 (5 agents, après fusion de la vague 1)

### F4b — Empire et Pays-Bas ; F4c — Îles britanniques ; F4d — Ibérie ; F4e — Italie (`cent-ans-mech`, un agent chacun)
Même recette que F4a. F4b porte le gros du volume. Cible globale : ≈ 130 factions, ≈ 250 provinces.

### F5 — IA féodale (`feat/fe5-ia`, `cent-ans-dev`)
- Dans `crates/ai`, remplacer les scores provisoires de F2 et F3 : réponse à l'appel de protection et d'ost, commise, concession, révolte ou changement d'allégeance, arbitrage. Poids dans `data/ai/feudal.json` × `ai_personality`.
- Doctrine « survie d'abord » (`data/ai/doctrines.json`) pour les factions de rang comté.
- Performances : banc `pb1` ≤ 1,5 × la référence. Sinon, évaluation allégée un tour sur deux pour les factions sans armée ni guerre.

## Vague 3 (3 agents, après fusion de la vague 2)

### F6 — Interface (`feat/fe6-ui`, `cent-ans-dev`)
- Pont : `campaign_sim_feudal.rs`, qui expose arbre, fil d'Ariane, obligations, aperçu d'escalade, objectifs et fiches de faction.
- Godot (spec § 6) :
  - choix de faction sur carte ;
  - filtre MF1 « Féodalité » (hachures, écu parti) ;
  - fil d'Ariane dans la fiche de province ;
  - panneau « Arbre féodal » (zone `SIDE_PANEL` d'`UiLayout`) ;
  - obligations ;
  - panneau « Qui peut entrer en guerre » ;
  - événements féodaux ;
  - objectifs ;
  - Codex « Vassalité » ;
  - tutoriel en 3 étapes.
- Tests : `game/tests/fe_ui_test.gd` et `smoke.gd` étendu ; `game/tests/fe_shot.gd` (choix de faction, filtre, arbre, aperçu d'escalade).

### F7 — Portraits (`feat/fe7-portraits`, `cent-ans-mech`)
- Pipeline DA2 existant pour les nouveaux souverains et héritiers, plus les variantes âgées des plus de 50 ans. `--dry-run` d'abord.
- Consignation de chaque lot dans `docs/budget.md`. Arrêt à 15 $.

### F8 — Équilibre (orchestrateur + 1 agent `cent-ans-dev`)
- `ai_replay` : parties IA contre IA de 100 ans sur N graines. Bandes cibles de la spec § 5 dans le rapport d'équilibre. Pas de régression de l'ADR 0085 (la guerre de Cent Ans éclate, déclenchée par la commise de Guyenne).
- Ajustement de `feudal.json` et `data/ai/feudal.json` seulement (pas de code sauf bogue).
- Puis une partie pilote du joueur.

## Critères de fin
- Tous les tests `feudal_*` actifs et verts ; `cargo clippy -D warnings` ; pytest ; `smoke.gd`.
- ≈ 130 factions jouables, ≈ 250 provinces, invariants du registre verts, relecture historienne faite pour chaque royaume.
- Banc `pb1` ≤ 1,5 × la référence ; bandes d'équilibre tenues ; ADR 0085 tenu.
- Dépense FE ≤ 15 $, consignée.
