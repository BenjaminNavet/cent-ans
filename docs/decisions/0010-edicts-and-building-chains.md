# ADR 0010 — Édits régionaux et chaînes de bâtiments (lot C4 « Total War »)

Date : 2026-09-24. Statut : accepté. Complète `docs/design/2026-09-24-analyse-total-war.md` § 2.1
(« Bâtiments en chaînes/arbres », « Édits régionaux »).

## Contexte

L'analyse Total War relève deux écarts : les bâtiments (25, `upgrades_from`/`required_building`) forment
des chaînes peu profondes et certaines familles empilent plusieurs paliers plutôt que de remplacer le
précédent (marché+halle+foire, paroissiale+abbaye+cathédrale à la fois) ; les édits régionaux (Rome II et
suivants) n'existent pas, seul le taux d'imposition (bas/normal/haut) module une province.

## Constat de départ

Le mécanisme de remplacement à la complétion (`buildings::resolve_construction` retire l'ancien bâtiment
de la chaîne quand le nouveau se termine, `build_blocker` vérifie `upgrades_from`) existait déjà pour
3 familles (palissade→mur de pierre→château→bastion, terrain de manœuvre→armurerie, marché→foire) : lot
C4 n'a donc pas eu besoin de créer cette mécanique, seulement d'étendre les chaînes de données et
d'ajouter les édits, mécanique neuve.

## Décision — chaînes de bâtiments

Trois chaînes étendues ou ajoutées, palier suivant remplaçant le précédent :

- **Commerce** : marché (1) → maison des métiers (2, `bld_guild_hall`, remplace `required_building` par
  `upgrades_from`) → foire (3, `bld_fair`, désormais palier 3 au lieu de 2).
- **Religieux** : église paroissiale (1, inchangée — trop référencée dans les 568 colonies pour être
  renommée sans risque) → **collégiale** (2, `bld_collegiate_church`, nouveau palier intermédiaire) →
  **abbaye** *ou* **cathédrale** (3, embranchement : les deux upgradent la collégiale, une seule peut être
  construite puisque la collégiale est consommée par la première des deux).
- **Production** : moulin à vent (1) → moulin à eau (2, `bld_water_mill`, exige désormais le moulin à
  vent en plus de sa rivière et de sa technologie).
- Fortification et terrain de manœuvre→armurerie : inchangées (déjà conformes).

Le palier et la technologie requise valent condition de « taille/type de colonie » (`settlement_kinds`
et `required_technology`, déjà en place) : pas de nouveau champ de taille de colonie, jugé hors budget
pour un gain marginal (le type de colonie discrimine déjà cité/ville/château/abbaye/village).

**Compatibilité** : aucun identifiant de bâtiment n'a changé, seuls `tier`/`upgrades_from`/
`required_building` ont bougé ; une sauvegarde ou une colonie de départ qui a plusieurs paliers d'une
même chaîne (ex. Paris : marché+maison des métiers+foire, paroissiale+abbaye+cathédrale, empilés depuis
avant ce lot) continue de charger et de calculer ses effets sans erreur — `effects_of` additionne
simplement la liste, la nouvelle règle ne restreint que les constructions futures. Un nettoyage des 568
fichiers de colonies pour ne garder que le palier le plus haut de chaque chaîne améliorerait l'équilibrage
mais est resté hors budget de cette session (voir `docs/wip/c4-edits-chaines.md`).

## Décision — édits régionaux

Nouveau module `sim-campaign::edicts`, calqué sur H3 « La Table » (`table.rs`, même patron : choix par
province, IA déterministe, bascule au défaut si les conditions ne tiennent plus) avec trois différences :

- **gratuit** : pas de coût récurrent (contrairement au régime alimentaire) ;
- **un seul actif par province**, condition = province **entièrement contrôlée**
  (`CampaignState::holds_whole_province`) plutôt qu'une liste de ressources/bâtiments requis : un édit est
  une politique de gouvernance, pas une production, elle n'a de sens que si le contrôleur tient toute la
  province ;
- **délai de changement** (`Edict::delay_turns`, 0 à 2 tours selon l'édit) : l'ancien édit reste actif
  pendant le délai (`EdictChoice::previous`), pas de mutation d'état requise pour « activer » un choix — il
  est dérivé à la lecture (`CampaignState::province_edict`) à partir du tour du choix et du délai.

Cinq édits (`data/edicts/`), plus `edict_none` (défaut, gratuit, sans effet) :

| Édit | Effet | Inspiration |
|---|---|---|
| Paix de Dieu | −8 désordre, +2 piété, −1 recrutement | mouvement de paix ecclésiastique (Xe s.) |
| Franchises marchandes | +12 % commerce, +4 richesse bourgeois, +3 désordre noblesse | chartes de commune, foires franches |
| Levée de la milice | +1 recrutement, −10 % coût, +garnison, +5 désordre, −richesse paysans | arrière-ban, milices communales |
| Aide féodale | +20 % impôt, +10 désordre (+5 paysans) | aide féodale exceptionnelle |
| Carême strict | +4 piété, −6 désordre clergé, +4 désordre noblesse, −3 satisfaction | observance stricte des jours maigres |

Les effets utilisent le vocabulaire `Effect` déjà partagé par les bâtiments, la table et les technologies :
`CampaignState::province_effects`/`settlement_effects` fusionnent les effets d'édit exactement comme ceux
d'un bâtiment ou d'un gouverneur (`EffectTotals::merge`), donc **aucune modification d'`economy.rs`** n'a
été nécessaire — tout ce qui lit déjà `province_effects` (taxe, commerce, désordre) bénéficie de l'édit
sans changement.

`ProvinceState::edict: Option<EdictChoice>` en `#[serde(default)]` : pas de changement de version de
sauvegarde ; une sauvegarde antérieure au lot C4 charge avec `edict: None` (édit par défaut).

## Alternatives rejetées

- Coût récurrent pour les édits (comme les régimes) : rejeté, un édit est une politique, pas une
  dépense de table ; le compromis coût/effet existe déjà par ailleurs (impôt, régimes).
- Condition par ressource/bâtiment (comme les régimes) : rejetée au profit de la condition unique
  « province entièrement contrôlée », plus proche de l'esprit Total War (un édit régional suppose
  l'autorité complète sur la région) et plus simple à lire dans l'UI.
- Renommer `bld_parish_church` en « Chapelle » pour coller à l'intitulé de la spec (« chapelle → église →
  abbaye ») : rejeté, l'identifiant est référencé dans 568 fichiers de colonies et plusieurs outils ; la
  collégiale (nouveau palier 2) suffit à former la chaîne à trois paliers sans ce risque.
