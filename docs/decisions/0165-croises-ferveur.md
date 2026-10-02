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
