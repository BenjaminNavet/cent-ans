# Plan — rapprochement Total War (session 6)

Source : `docs/design/2026-09-24-analyse-total-war.md` (tableau comparatif, lots T1-T10) complété par un examen des captures (`docs/img/visuel/v4b_*.png`, `hud-campaign.png`, `godot-battle-f5.png`).
Suivi : `docs/wip/tw.md`. Hors périmètre : colonies (C2c-C7), portraits.

## Constat complémentaire (visuel)

L'analyse juge les batailles « au niveau TW » sur le plan des règles ; c'est vrai de la simulation, pas de l'image :
- soldats et chevaux = mannequins procéduraux (membres cylindriques, chevaux sans tête lisible) ; c'est l'écart visuel n° 1 face à un TW ;
- aucun repère flottant par régiment (TW : bannière d'unité avec icône de classe, effectif, état, sélection) ; seuls un mât et un nombre ;
- cartes d'unités textuelles (lignes Moral/Fatigue/Formation) au lieu de vignettes illustrées compactes ;
- écran de fin de bataille minimal ;
- sur la carte de campagne : pas de minicarte, pas de brouillard, pas d'aire de déplacement visible (TW : zone atteignable colorée), étiquettes de province en texte brut.

## Mécaniques à préserver
Régimes et Carême, médecine, codex, monnaie et mutations, rançons, ordres de chevalerie, ordres du chef historiques, dynasties, papauté/Schisme, guerre de Cent Ans « vivante » (F4/G2), objectifs historiques (voir § 4 de l'analyse).

## Lots

| Lot | Domaine | Contenu | Coût | Vague |
|---|---|---|---|---|
| B1 | bataille/visuel | Nouveaux maillages soldats (fantassin, archer, chevalier) et **cheval** modelés sous Blender, compatibles avec l'animation par shader existante (pivots hanche/épaule) ; LOD conservé | L | 1 |
| B2 | bataille/UI | Bannières flottantes d'unité à la TW (icône de classe, effectif, moral, sélection, survol) ; cartes d'unités en vignettes illustrées compactes ; écran de fin de bataille mis en scène (T2) | M | 1 |
| C1 | campagne/UI | Minicarte de campagne + brouillard de guerre léger (T1) | M | 1 |
| C2 | campagne | Zone de contrôle (T3) + aire de déplacement atteignable affichée à la sélection d'une armée | M | 2 |
| B3 | bataille/audio+caméra | Musique dynamique par intensité (T4) ; caméra de suivi d'unité / du général (T6) | M | 2 |
| B4 | bataille/visuel | Effets : poussière des charges, traînées de flèches, fumée des bombardes, sang discret, bannières de chef plus hautes | M | 2 |
| C3 | campagne/UI | Arbre familial graphique (T5) ; fiche de général à la TW (compétences, suite) si les données existent | M | 3 |
| C4 | campagne | Édits régionaux / chaînes de bâtiments (T7), **après C4 colonies** | M-L | 4 |
| C5 | campagne | Routes commerciales visibles et accords (T8) | M | 4 |
| C6 | campagne | Agents à la Medieval II (espion, prédicateur, émissaire) (T9) | L | 5 |

Décisions :
- Thème d'interface : on garde le registre parchemin (identité du jeu, travail ui-tw), mais on vise la **densité** TW (vignettes, icônes, moins de texte) plutôt que ses couleurs sombres.
- Pas de batailles navales ni de cinématiques (cadrage v1 confirmé).
- Les lots qui touchent `ProvinceState` (bâtiments, recrutement) attendent la fusion de C4 colonies.
