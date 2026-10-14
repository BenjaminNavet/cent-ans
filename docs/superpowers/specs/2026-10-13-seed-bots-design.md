# Seed bots : bots de test à graine sur `core/` — design

Date : 2026-10-13. Statut : validé en brainstorming, en attente de relecture de la spec.

## Contexte

Le joueur veut aller vers un « studio autonome » : duos codeur + relecteur, médiateur de fusion,
directeur artistique qui compare les captures à des références, et bots de test à graine lancés après
chaque grosse fonctionnalité. Ce chantier est découpé en quatre sous-projets indépendants :

1. **Seed bots** (ce document) ;
2. orchestration (duos codeur/relecteur, médiateur, backlog) ;
3. directeur artistique (banque de références, notation des captures) ;
4. GDD + backlog exécutable.

Choix faits en brainstorming (non rediscutés) :
- Le studio autonome est **indépendant du moteur** : il est conçu sur `core/` (Rust, sans moteur) et ne
  dépend pas du verdict P0 Bevy contre Godot (`2026-10-13-proto-bevy-design.md`). Les bots côté
  client (interface) viendront après ce verdict et réutiliseront les oracles d'ici.
- Les seed bots sont le **premier** sous-projet : c'est le filet de sécurité qui rend l'autonomie sûre.
- Approche **A** : un crate Rust dans `core/` (pas de harnais Python, pas de seuls tests de propriété).

Ce qui existe déjà et sert de socle : la simulation est pure et déterministe
(`état + ordres + graine → nouvel état`, `CampaignRng`), `submit_order` renvoie un `Result`,
`save_json`/`load_json`, `end_turn_with(planner)`, `agents::derived_rng(seed, turn, agent, salt)`,
les API de prévisualisation et de prérequis (`preview.rs`).

## Périmètre

Dans le périmètre : parties complètes de campagne jouées sans moteur par 8 bots, quatre familles
d'oracles, rapports reproductibles, intégration au flux de travail des agents.

Hors périmètre (YAGNI) : batailles jouées en 3D (les batailles restent auto-résolues par `core/`),
bots d'interface (après le verdict P0), réduction automatique des journaux d'ordres.

## Architecture

Nouveau crate `core/crates/seed-bots`, dépendant de `sim-campaign` et `data-model`, sans les modifier
sauf l'ajout d'`invariants.rs` (voir Oracles).

| Module | Rôle |
|---|---|
| `bot.rs` | Trait `Bot` : `fn plan(&mut self, view: &BotView, rng: &mut CampaignRng) -> Vec<Action>`. |
| `action.rs` | `Action` = un `Order` ou une action hors `Order` (diplomatie, recherche, édits, réponses aux événements), soumise par l'API publique que l'interface utilise déjà. |
| `view.rs` | `BotView` : vue en lecture seule bâtie sur les requêtes de l'interface (le bot ne voit que ce qu'un joueur voit). |
| `bots/` | Un fichier par bot. Chaque bot génère des actions candidates légales puis les filtre selon sa personnalité. |
| `oracles/` | Un fichier par famille : `fn check(ctx: &OracleContext) -> Vec<Finding>`. |
| `runner.rs` | Charge `data/`, joue pour chaque (graine, bot, faction) T tours : plan → soumission → oracles → `end_turn` → oracles. Un fil par bot. |
| `report.rs` | Rapport JSON + résumé Markdown en français, déduplication des constats. |
| `main.rs` | Binaire `seed-bots` (`run`, `run --quick`, `replay`). |

### Les 8 bots

1. **random-legal** : actions légales tirées au hasard ;
2. **agresseur** : déclare la guerre, attaque, assiège ;
3. **économe-bâtisseur** : construit, lève les impôts, recrute peu ;
4. **diplomate** : alliances, paix, mariages, vassalisation ;
5. **tortue** : défend, ne s'étend jamais ;
6. **marin-explorateur** : embarque, navigue sur les fleuves, débarque loin ;
7. **chaos** : actions aux limites (scinder, fusionner, annuler, licencier, ordres contradictoires) ;
8. **miroir** : l'IA du jeu joue la faction du joueur (`plan_agents` / planificateur de l'IA).

La faction jouée est tirée de la graine parmi les factions jouables (France, Angleterre, Bourgogne
toujours couvertes sur la passe longue).

## Oracles

1. **Panics et blocages** : `catch_unwind` autour de chaque tour ; tour (joueur + IA) au-delà d'une
   limite (5 s par défaut en mode optimisé) signalé ; aller-retour `save_json → load_json → save_json`
   identique tous les 10 tours et en fin de partie.
2. **Invariants de règles** : `fn check_invariants(&CampaignState, &GameData) -> Vec<Violation>` dans
   `sim-campaign/src/invariants.rs`, réutilisable par `cargo test` et par tout client futur. Liste
   initiale : propriétaire unique par colonie ; armée à un seul endroit ; effectifs ≥ 0 ; trésor fini ;
   aucune armée d'une faction disparue ; références (provinces, personnages, titres) valides.
3. **Ordres légaux refusés** : un bot ne propose que des actions que l'API de prévisualisation ou de
   prérequis déclare possibles ; un `Err` de `submit_order` sur l'une d'elles est un constat
   (incohérence entre ce que l'interface promet et ce que les règles acceptent).
4. **Dérive d'équilibre** (passe longue seulement) : métriques par partie (révoltes, faillites, durée
   des guerres, factions éliminées, part de territoire de la première faction), agrégées par bot sur
   N graines et comparées à des bandes dans `data/balance/seed_bots_bands.json` (schéma dans
   `data/schemas/`). Bandes initialisées à partir d'une première passe de référence, puis réglées à
   la main.

Chaque constat porte : oracle, gravité (`bloquant` pour 1-3, `avertissement` pour 4), graine, bot,
faction, tour, message, et le journal d'ordres jusque-là.

## Rapports et reproduction

- Sortie : `target/seed-bots/<date-heure>/report.json` + `report.md` (résumé en français).
- `seed-bots replay <constat.json>` rejoue la partie à l'identique et s'arrête au tour fautif.
- Déduplication par (oracle, signature du message) avec nombre d'occurrences.

## Exécution et coût

Mesure de départ : une fin de tour coûte ~100-150 ms dans le cœur Rust compilé en optimisé ;
`cargo test` (non optimisé) est 10 à 20 fois plus lent. D'où trois niveaux :

| Quand | Quoi | Budget |
|---|---|---|
| Chaque commit (`cargo test`) | Tests unitaires des oracles sur de petits états construits à la main ; aucune partie jouée. | < 1 s |
| Fin de lot / avant fusion dans `main`, et CI | `seed-bots run --quick` en optimisé : 8 bots × 1 graine × 20 tours (mesuré : 20-30 s), oracles 1-3. | ≤ 30 s |
| Après une grosse fonctionnalité | `seed-bots run` : 8 bots × 4 graines × 150 tours (mesuré : ~12 min ; les 3 factions jouables couvertes), oracles 1-4, en arrière-plan. | ≤ 15 min (plafond dur, rapport partiel au-delà) |

Un test `#[ignore]` reprend la passe rapide pour un lancement à la main (`cargo test -- --ignored`).
Le premier lot mesure les durées réelles et ajuste graines et tours pour tenir ces budgets.

### Validation par le joueur de la passe longue

La passe longue est **lancée d'office** en arrière-plan (règle changée le 13/10 : elle ne coûte
rien, ADR 0348 addendum) :
1. À la fin d'une grosse fonctionnalité, l'agent lance la passe longue sans demander.
2. À la fin de la passe, l'agent présente le résumé `report.md` (constats bloquants, dérives
   d'équilibre) ; le joueur valide quels constats entrent au backlog
   (`docs/wip/seed-bots-findings.md`), lesquels sont ignorés ou demandent un réglage des bandes.

La passe rapide (fin de lot, CI) reste automatique : elle est courte et ne signale que des bogues
francs.

## Vérification

- Tests unitaires de chaque oracle (un état valide passe, un état corrompu à la main est signalé).
- Déterminisme : deux passes rapides de même graine produisent un `report.json` identique.
- Rejeu : un constat provoqué volontairement (invariant cassé en test) se rejoue au même tour.
- `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`.

## Livrables

- Crate `core/crates/seed-bots` (8 bots, 4 familles d'oracles, binaire `seed-bots`).
- `sim-campaign/src/invariants.rs`.
- `data/balance/seed_bots_bands.json` + schéma.
- Passe rapide branchée en CI ; mention dans `docs/architecture.md` et `CLAUDE.md` (commandes).
- ADR : crate, oracles, niveaux d'exécution, validation par le joueur de la passe longue.
- Première passe longue de référence (lancée le 13/10), qui fixe les bandes d'équilibre.

**Écarts constatés à l'implémentation** : pas de type `Action` (les bots jouent des `Order`), défauts de passes réduits par les mesures (rapide 20 tours, longue 4 graines), bandes en JSON ; détails dans `docs/wip/seed-bots.md`.
