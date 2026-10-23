# 0748 — Journal des causes (« pourquoi ai-je perdu / gagné ? »)

Statut : accepté (lot CAUSES, 23/10/2026).

## Contexte

Le juge game design (`out/juge/iter-02-design.md`, constat 10 et chantier 5) note que le jeu a des outils
locaux (prévision, ventilation du revenu, explication de traité) mais aucun « pourquoi » global : défaite de
bataille, perte de province, banqueroute, révolte, guerre subie, succession ne disent pas ce qui les a causés.
La chronique existante (`ChronicleLog`, onglets Décisions / Journal / Lettres) ne garde que du texte français
formé par le cœur, sans structure ni chiffres.

## Décision

1. **Cause structurée dans le cœur.** `sim-campaign::causes` définit `Cause` : `kind` (`CauseKind`, énumération
   stable snake_case), `subject` (faction touchée), `actor` (faction adverse ou bénéficiaire), `province`,
   `severity` (0-100, sert au classement « 3 causes principales »), `factors` (`Factor { key, value, weight }`,
   valeurs entières — pourcentages ou points —, triés par poids décroissant) et `reason` (`AiReason`, clé de
   doctrine ou d'objectif quand la décision est celle de l'IA). Aucun texte dans le cœur : seulement des clés.
2. **Journal borné et sauvegardé.** `CampaignState::causes: CauseJournal` (`VecDeque<CauseRecord>`, au plus
   `MAX_RECORDS` = 300 ; les plus anciennes sortent). `CauseRecord` ajoute `id` (croissant), la date
   (tour, saison, année). Le champ est `#[serde(default)]` ; `STATE_VERSION` passe de 9 à 10 avec une
   migration explicite (`causes` vide) pour les sauvegardes v9.
3. **Câblage par l'événement.** `GameEvent` gagne `cause: Option<Box<Cause>>` (constructeur `.cause(c)`) et
   `cause_id: Option<u32>`. En fin de tour, `causes::harvest` verse les causes portées par les événements dans
   le journal, assigne l'identifiant et le pose sur l'événement (`cause_id`) : la notification renvoie ainsi à
   sa cause. Les producteurs n'ajoutent qu'un `.cause(...)` à l'événement qu'ils émettent déjà. Les batailles
   passent par `causes::note_battle` (chemin commun `apply_battle_result`, auto-résolution et bataille jouée).
4. **Producteurs du lot** : bataille (perdue/gagnée, joueur impliqué), place prise (siège, reddition, famine),
   banqueroute (ventilation du budget), révolte et avis de grogne (termes du mécontentement de la province),
   guerre déclarée (motif, rupture de trêve, serment trahi), alliance rompue, succession.
5. **Raison de l'IA.** Quand l'IA déclare une guerre, `AiReason` nomme la doctrine ou l'objectif
   (ultimatum refusé, couronne revendiquée, appel de l'allié, suzerain perdant, ambition). Les ordres de l'IA
   sont planifiés sur un état en lecture seule : la raison est donc recalculée à l'application de l'ordre
   (`apply_ai_order`) par les mêmes fonctions pures que le planificateur. Limite assumée : la doctrine de
   recrutement (crate `ai`) n'est pas encore journalisée.
6. **Textes en données.** `data/ui/causes.fr.json` (schéma `data/schemas/causes.schema.json`) : modèles de
   phrase par `kind`, libellés des facteurs et des raisons, avec `{valeur}`, `{lieu}`, `{acteur}`. `game-api`
   (`causes.rs`) formate : vue filtrée (« concerne le joueur », par faction, par province), « 3 causes
   principales » d'une fin de partie, cause d'un événement par `cause_id`. Le client ne fait que le rendu.
7. **Client.** Onglet « Causes » du panneau Chronique (accessible aussi en fin de bataille et en fin de
   partie) ; écrans de défaite et de victoire : les 3 causes principales ; infobulle sur les notifications
   portant un `cause_id`.

## Conséquences

- Coût mémoire borné (≤ 300 enregistrements de quelques facteurs) ; sauvegardes v9 migrées sans perte.
- Aucune règle de jeu ne change : le journal ne fait que lire l'état au moment de l'événement. Déterministe
  (pas d'aléa, pas d'horloge).
- Les causes sont des explications a posteriori calculées sur des quantités déjà présentes ; les facteurs
  sont classés par poids, pas par causalité stricte : le texte dit « facteurs » et non « preuves ».
- Évolutions possibles : doctrine de recrutement de l'IA, causes de pertes de trésorerie par armée, terrain
  de bataille.
