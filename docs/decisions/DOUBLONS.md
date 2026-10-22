# Numéros d'ADR en double

Constat du 2026-10-22 : cinq numéros d'ADR sont portés par deux fichiers chacun. Dix fichiers sont
concernés, sur 300 ADR. Rien n'a été renommé ni supprimé : ce document décrit l'existant et la
convention déjà employée dans `INDEX.md`.

## Liste

| Numéro | Fichier « principal » (sans suffixe dans `INDEX.md`) | Fichier « bis » (suffixe `bis` dans `INDEX.md`) |
|---|---|---|
| 0117 | `0117-garde-du-seigneur.md` : garde du seigneur (lot OMR R3) | `0117-lanceur-depuis-les-sources.md` : lanceur depuis les sources (macOS, Linux, Windows) |
| 0146 | `0146-regiments-doubles.md` : deux fois plus de régiments par bataille | `0146-sommation-de-paix-du-suzerain.md` : sommation de paix du suzerain |
| 0149 | `0149-fal-ai-hors-plafond-campagne-tob.md` : fal.ai hors plafond v1 pour TB | `0149-paquet-de-relief-automatique.md` : le paquet de relief se met à jour seul |
| 0205 | `0205-plancache.md` : cache de planification explicite | `0205-retrait-village-bataille.md` : retrait du village de bataille B5 |
| 0357 | `0357-architecture-client-bevy.md` : architecture du client Bevy | `0357-echelle-de-jeu-carte-composee.md` : échelle de jeu de la carte composée |

Statuts dans `INDEX.md` : 0117 et 0117 bis acceptés ; 0146 et 0146 bis sans statut (n/d) ; 0149
remplacée par l'ADR 0152 et 0149 bis sans statut ; 0205 et 0205 bis acceptés ; 0357 et 0357 bis acceptés.

## Références ambiguës constatées

- `CLAUDE.md` : « ADR 0357 » désigne l'architecture du client Bevy (0357, fichier principal).
- `CLAUDE.md` et `tools/launch.sh` : « ADR 0117 » désigne le lanceur depuis les sources (0117 bis).
- **0149** est cité dans les deux sens (`git grep "ADR 0149"`) :
  - pour le chantier fal.ai hors plafond TB (0149 principal, remplacé par 0152) : `docs/budget.md` (enveloppe TB),
    `docs/design/2026-10-02-campagne-tob.md` ;
  - pour le paquet de relief qui se met à jour seul (0149 bis) : `docs/geo.md` (§ « Mettre le paquet à jour »),
    `data/map/relief_hosting.json`, `.gitignore`, `docs/audit/equipe-art-2026-10-09/11-monde-terrain.md`.
  - `docs/decisions/0212-paquet-de-modeles-generes.md` mêle les deux usages dans le même paragraphe.
- Les autres citations de numéros doubles dans `docs/wip/` et `docs/design/` ne sont pas recensées ici.

## Convention proposée (non appliquée)

Pour une citation sans ambiguïté, écrire le numéro avec son suffixe (« ADR 0149 bis ») ou son titre.
Pour les nouvelles décisions, prendre le numéro suivant le plus haut (0711 à la date du 2026-10-22).
Un renommage des fichiers en doublon exige un accord explicite : il casserait les liens existants.
