# ADR 0165 — Faction croisée : une colonie pour base, la Ferveur pour assise

Date : 2026-10-02. Chantier JR, spec `docs/superpowers/specs/2026-10-02-jr-croises-jerusalem-design.md`.

## Contexte

Le joueur veut une faction de croisés à part entière, avec une mécanique propre et Jérusalem pour
objectif. En 1337 il n'existe plus d'État latin en Terre sainte ; `prov_jerusalem` est aux
Mamelouks. Trois voies : (a) une faction sans terre du tout, (b) découper une nouvelle province,
(c) une faction basée dans une colonie d'une province existante.

## Décision

- **(c)** : `fac_crusaders` possède la seule colonie `set_limassol` dans `prov_cyprus`. La
  propriété par colonie et la survie « une colonie ou une armée » existent déjà ; aucune
  régénération géographique, aucun changement des règles communes.
- La mécanique propre est une **jauge de Ferveur** portée par un état optionnel de
  `CampaignState`, entièrement paramétrée par `data/rules/crusade.json` (faction, province but,
  barème). Le Rust ne nomme ni la faction ni Jérusalem.
- Les contingents du passage réutilisent les types d'unités existants.
- Objectifs par le bloc `victory` de la faction (chemin France/Angleterre/Bourgogne).

## Conséquences

- (a) écartée : revenu, recrutement, IA et élimination supposent tous une assise ; trop de règles
  communes à dérouter pour une seule faction.
- (b) écartée : régénérer les artefacts de 443 provinces pour une île déjà présente.
- La mécanique est réutilisable pour une autre croisade en changeant le fichier de règles ; une
  seule croisade à la fois (un seul état).
- Faction uchronique : données marquées `uncertain` avec note, à l'écart de l'audit historique.
- 0 $ : armes et portrait par les moyens existants sans génération payante.

## Ajouts d'équilibrage (JR4, JR4b)

- **Vœu de l'IA** : menée par l'IA, la faction des règles ne fait pas la paix avec le maître de la
  province cible (le joueur reste libre, au prix de la ferveur).
- **Levée de secours** (`crusade.json` § `relief`) : la place de Terre sainte assiégée par la
  croisade reçoit une fois par siège quelques unités de son maître. Sans elle, Jérusalem tombait
  sur 4 graines sur 5.
- **Budget de départ** (`data/settlements/rules.json` § `starting_budget`, règle commune) : au
  setup, une faction d'au moins 5 provinces dont le solde dépasse −15 % de ses recettes renvoie
  ses garnisons les plus chères (jamais la dernière unité d'une colonie ni la capitale). Cause :
  coûts de garnison fixes par colonie contre recettes proportionnelles à la population, d'où une
  faillite structurelle des grandes factions de l'extension OM (Mamelouks −21 %, Byzance −42 %,
  Horde −80 %). Factions saines inchangées. Restent en déficit : Mérinides, Hafsides, Lituanie,
  Serbie (populations OM ou coût des colonies à revoir, hors JR).
  Ajout LR-15 : deux paliers de plus, dans l'ordre, pour une faction encore au-delà du seuil une
  fois ses garnisons au plancher : armées de campagne de départ (`min_field_units` 1), puis
  garnison de la capitale (`min_capital_units` 2). Populations du Maghreb mérinide et hafside
  relevées ×2,5 (sous-estimées : Fès 15 000 citadins). Les quatre factions passent sous −15 %.
- **Rassemblement sans cité** (IA, `ai/src/campaign.rs`, règle `cityless`) : une faction qui ne
  tient aucune cité recrute dans ses bourgs et châteaux et en fait sortir le surplus, en gardant
  la garnison de départ du type de chaque place. Voulu pour toute faction réduite à des châteaux,
  pas seulement la croisade ; elle ne vide jamais sa dernière place (test
  `a_cityless_realm_never_empties_its_last_place`).
- **Correctifs JR5** : évènements `public` (champ explicite de `GameEvent`, plus de marque dans
  le texte) et `loss` (ton) ; levée de secours publique ; sortie : l'assiégeant est l'attaquant ;
  capitale rétablie à la perte de la cible ; gain et prestige de la cible à la première
  délivrance seulement, chaque place de Terre sainte comptée une fois ; contingents dans la
  limite de garnison (surplus : autres ports, puis armée au port) ; budget de départ mesuré au
  niveau de difficulté neutre.
