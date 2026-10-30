# XXXX — ECO-R1 : l'impôt consenti (assemblées, subsides, concessions)

Statut : accepté (30/10/2026, lot ECO-R1ab de la boucle juge 2). Numéro à fixer à la fusion.

## Contexte
L'audit d'économie politique (`docs/audits/2026-10-29-economie-politique.md`, § 6 R1) a mesuré, à la sonde séculaire
(`eco_series`, IA aux commandes de la France), des impôts exceptionnels à 0,7-2,5 % des recettes et une armée à 12-16 %
des dépenses : le jeu modélise une cour qui gère une trésorerie, pas un État qui naît de la guerre par l'impôt consenti.
Le joueur levait l'aide, la gabelle ou le fouage d'un ordre (`LevyExtraordinary`, ADR 0777) contre un peu de grogne :
l'argent n'était jamais une décision politique.

## Décision
1. **Données** (`data/rules/assemblies.json`, schéma `assemblies.schema.json`, `data-model::AssemblyRules`) : assemblées
   (États de langue d'oïl et d'oc, Parlement, états de Bretagne, membres de Flandre) avec maison, provinces ou régions
   couvertes, humeur de départ, délai minimal entre deux convocations (≥ 4 saisons) ; nécessités (guerre défensive,
   invasion, rançon, campagne offensive, croisade) avec multiplicateur de subside et bonus d'accord ; paliers de demande ;
   formes de subside (aide, gabelle, fouage, quinzième et dixième) avec poids urbain/rural et possibilité de permanence
   (1360-1363) ; catalogue de concessions. Sans assemblée dans les données, tout reste comme avant.
2. **État** (`CampaignState::assemblies`, `serde(default)`, absent d'une sauvegarde intacte) : par assemblée, humeur,
   sessions, promesses, subsides en cours, griefs ; formes rendues permanentes par faction.
3. **Ordres** : `ConveneAssembly { assembly, necessity, ask, form?, permanent? }` et `AnswerAssembly { session, accept_terms }`,
   refusés avec `AssemblyError` (nécessité absente de la situation, délai, session ouverte, forme indisponible...).
   `LevyExtraordinary` est refusé (`FinanceError::AssemblyRequired`) à une maison qui a une assemblée ; les autres maisons
   gardent les impôts de `finance.json`.
4. **Session** (`sim-campaign/src/assembly.rs`) : à la convocation l'assemblée répond la saison suivante. Accord =
   humeur + légitimité provisoire (prestige du souverain × poids) + bonus de nécessité + modificateur de palier − poids des
   subsides en cours − agitation moyenne des provinces couvertes − promesses trahies mémorisées (− pénalité si impôt permanent).
   Accord ≥ seuil : subside accordé ; entre les deux seuils : accord sous conditions (1 à 3 concessions tirées, par coût
   croissant, parmi les demandes de l'assemblée), le joueur répond (`AnswerAssembly`) dans une fenêtre, sinon la session
   tombe ; en dessous : refus (humeur perdue, cause `assembly_refused`). Le calcul est déterministe et publié
   (`assembly::forecast`) pour l'interface et l'IA.
5. **Perception** : le subside est versé **par saison** pendant un terme, réparti sur les provinces couvertes (part et
   impôt ordinaire de chacune mémorisés) ; le revenu passe par le registre `levied` du budget (donc `levy_income`, hors
   `receipts`, comme les anciens impôts). La grogne est locale : unrest = fardeau (part/ordinaire) × `unrest_per_burden`
   × poids de la forme (urbain/rural selon la part de bourgeois), plafonné (`population.rs` appelle `assembly::province_unrest`).
6. **Concessions** : `strong_money` (le cœur laisse affaiblir la monnaie mais l'acte trahit la promesse), `elected_officers`
   (plafond de la maison du roi, `finance::household_percent`), `dismiss_counsellor` (renvoi immédiat via `council::dismiss`),
   `end_war` (trêve dans le délai, sinon promesse trahie), `grant` (privilèges, remises : coûts seuls : prestige, rentrées
   ordinaires cédées en obligation `Remission`, grief négatif). Promesse tenue : humeur +10 ; trahie : humeur −30, grief
   urbain, événement et cause `promise_broken`, mémoire dans le vote.
7. **Permanence** : une forme rendue permanente est un impôt ordinaire (part du revenu brut, `faction_income`), au prix
   d'une agitation constante ; l'ordre `permanent` coûte des points d'accord.
8. **Causes** (ADR 0754) : `assembly_refused`, `promise_broken`, `subsidy_granted` dans le système existant, textes dans
   `data/ui/causes.fr.json`. Événement `EventKind::Assembly`.
9. **IA minimale** (`assembly::ai_assembly_orders`, branchée dans `ai/src/campaign.rs`) : répond aux conditions selon un
   plafond de coût, convoque quand le trésor prévisible passe sous N saisons de revenu et qu'une nécessité existe.
   R1-c (maltôte, honneur, prévision) l'affinera.
10. **Équilibre** : l'ordinaire est resserré dans `economy.json` (mesures dans `docs/wip/eco-r1ab.md`).

## Conséquences
- L'interface Finances (ADR 0780) montre toujours les boutons de l'ancien impôt : refusés pour une maison à assemblée
  jusqu'à R1-d (onglet « États et Parlement »).
- Les tests qui levaient l'impôt de France (`eco2_finance`) utilisent des données sans assemblée.
- La ponction d'épargne (ADR 0749/0788) n'est pas retirée : à mesurer après R1-c.
- Le lot XP-POURQUOI ajoute d'autres genres de causes dans `game-api/src/causes.rs::explains` : fusionner les deux listes.
