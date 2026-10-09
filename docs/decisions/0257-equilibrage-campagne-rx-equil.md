# ADR 0257 — Équilibrage de campagne après la revue d'experts (lot RX equil)

Date : 2026-10-09. Statut : accepté. Complète les ADR 0085 (EQ6), 0100, 0245 (victoire) et 0246 (sonde).

## Contexte

La revue RX (`docs/wip/rx/mecaniques.md` #4-#11, `ia.md`) mesurait : 42-54 guerres actives en moyenne, 15 % de
guerres redéclarées par la même paire en 12 tours, durée médiane des guerres collée au plancher
`min_war_turns` (20), trésors thésaurisés, arbre de 45 technologies saturé par les petites factions, aucune fin
de campagne pour les 146 factions jouables sans bloc `victory`. Mesures avec `campaign_probe` (ADR 0246),
120 tours, `--full`, IA pour toutes les factions.

## Décision

Données (`data/`) :
- `ai/diplomacy.json` : `peace_truce_turns` 8 -> 20 ; `war.rest_turns` 24 (tours entre deux déclarations d'une
  même faction, avant : constante 12) ; `war.max_enemies` 3.
- `rules/economy.json` : opulence progressive (`opulence_high_seasons` 12, `opulence_high_percent` 15, en plus
  des 20 % au-delà de 6 saisons) ; `research_slowdown_percent_per_tech` 3.
- `rules/feudal.json` : `default_end_year` 1453.

Règles (`core/`) :
- `EconomyRules::opulence` (une seule formule pour l'entretien et `starting_fit`) ; recherche ralentie de
  1 / (1 + 3 % x technologies possédées) dans `research_points_per_turn` (l'interface affiche déjà ce nombre).
- Diplomatie IA : `war.rest_turns` et `war.max_enemies` remplacent `WAR_REST_TURNS` si non nuls. Un prétendant
  qui presse sa revendication principale (`pretender_ready`) n'est pas soumis au plafond : la guerre de Cent Ans
  ne l'attend pas (avec le plafond appliqué à tous, FR-EN tombait à 38-52 %).
- Victoire : sans bloc `victory`, la campagne se termine (`Ended`) après `default_end_year`.
- Sonde : durée médiane des guerres terminées et part de redéclarations en 12 tours ou moins.

## Mesures (120 tours, `--full`)

| Mesure | Avant (main) | Après |
|---|---|---|
| Graines | 1 à 6 | 1 à 6 |
| Guerre FR-EN | 66 / 92 / 72 / 76 / 78 / 63 % | 66 / 53 / 68 / 66 / 62 / 66 % |
| Guerres actives (moyenne) | 41-55 | 34-45 |
| Guerres déclarées | 230-379 | 180-301 |
| Redéclarations <= 12 tours | 5-14 % | 0-5 % |
| Durée médiane des guerres | 20 t | 20-21 t |
| Factions éliminées / 177 | 19-24 (11-14 %) | 17-27 (10-15 %) |
| Banqueroutes | 84-126 | 76-166 |

Les variantes écartées (même sonde) : trêve 30 + `min_war_turns` 28 (FR-EN 50-75 %, guerres actives 46-55, aucun
gain) ; plafond de 2 ennemis pour tous (FR-EN 40-52 %) ; plafond 4 / repos 20 (FR-EN 38-78 %, trop bruité).

## Conséquences

- Bande FR-EN 55-75 % tenue sur 5 graines sur 6 (graine 2 : 53 %, deux points sous la bande) contre 3 sur 6 avant (graines 1 à 6 mesurées sur main : 66 / 92 / 72 / 76 / 78 / 63 %).
- Non atteints : guerres actives (toujours 34-45, soit un quart des factions, cible non chiffrée mais encore
  « permanente ») ; durée médiane toujours égale au plancher (la paix est acceptée dès qu'elle est permise : il
  faudrait une règle de durée liée au score de guerre, non faite) ; banqueroutes inchangées (quelques factions
  en dette chronique, p. ex. Byzance ; 0,5 % des tours-factions).
- Éliminations : la cible (<= 20 % à 120 tours) est déjà tenue (10-15 %) ; la cible à 464 tours (35-38 %) n'a pas
  été remesurée (trop long sur machine chargée).
- Trésors : l'opulence progressive ne joue qu'au-delà de 12 saisons de revenu, absent à 120 tours (France
  0,6 saison) ; son effet attendu est tardif, non mesuré. Pas de nouveaux débouchés (fondations, mécénat) : lot M.
- Technologies : ralentissement à valider sur 464 tours (techs possédées par faction) ; tiers 4-5 non ajoutés.
- Ordre public (#9) : non modifié, le seuil 75 est lié à la bande FR-EN (ADR 0100) ; mesure à rapporter par
  province-décennie plutôt que par partie. Journal (#10) : `Income` et `BuildingCompleted` sont filtrés au joueur
  volontairement (`economy.rs`, `buildings.rs`) ; la dispersion mesurée vient du seul royaume de France.
- Croisés/mamelouks (ia #7) : aucune règle, guerre voulue (ADR JR).
