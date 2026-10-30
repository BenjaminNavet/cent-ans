# Chantier KF — kits de figurines par emplacements et par époque

**État (30/10/2026) : idée cadrée, rien de commencé.** Pas de branche, pas d'ADR. À reprendre par l'étape 1.

## But

1. **Variété** : aujourd'hui une figurine a au plus 6 variantes (masque de 6 bits par pièce,
   `client/src/battle/units/skinned.wesl` l. ~342, `tools/blender_scripts/battle_skinned_figures.py`) et
   39 types d'unités se partagent ~23 figurines. Viser des centaines de combinaisons par figurine en
   tirant **une option par emplacement** (tête, corps, extras, arme…) au lieu d'une variante globale.
2. **Époque** : l'équipement suit la date de campagne (aujourd'hui seule la disponibilité change,
   `available_from` des `data/unit_types/*.json`). Repères : vers 1340 bassinet + camail + jupon,
   gambison/aketon ; vers 1380 brigandine, coat of plates, heaume/bassinet à visière ; vers 1420-1450
   harnois blanc, salade, barbute, armet, chapel de fer.

## Contraintes connues

- Pas de soldat généré en 3D (ADR 0136) : pièces = recettes Blender par script, sur les os existants
  (rig `human` 22 os, `cavalry`, ADR 0014), 3 LOD par pièce dans le plafond actuel
  (≈ 2 100-2 400 / 500 / 230 tri à pied ; figurines fines 9-17 k au LOD0, ADR 0089).
- Un seul maillage fusionné par figurine (instanciation) : les emplacements restent des pièces
  repliées en un point par le shader, comme le masque actuel.
- Règle rendu seulement : le choix du kit est du client ; la date vient de l'état de campagne.
- Héraldique déjà procédurale (`heraldry.rs`, surcot + faces d'écu) : toute nouvelle forme de
  bouclier doit exposer une face d'armoiries.
- Pièces à composer = LOD **par pièce** obligatoire (sinon les soldats restent lourds au loin).

## Plan proposé

1. **ADR** « kits par emplacements » : format de données, encodage par sommet (emplacement, option)
   à la place du masque, tirage par soldat (hachage stable par emplacement), pondérations.
2. **Système sans nouveau modèle** : schéma `data/schemas/` + `data/art/unit_kits.json` (type
   d'unité × époque → emplacements → options pondérées) ; export Blender marquant chaque pièce ;
   shader ; sélection par date. Redistribuer les pièces existantes → mesurer le gain de variété
   (dump d'état + une capture de contrôle).
3. **Pièces par époque** (lots mécaniques, Sonnet) : salade, barbute, armet, chapel de fer,
   brigandine, jupon, harnois blanc ; formes d'écu (écu, targe, pavois) avec face d'armoiries.
   Essai unique : un casque par image → 3D puis réduction, avant d'en faire une règle.
4. **Petits ajouts** : pennons de lance (< 300 tri), armoiries sur caparaçon unique, drapeaux
   génériques (écartelé, bandes) aux couleurs du camp pour le commun.
5. **Types d'unités à étudier** (design, pas rendu) : billmen, pavoisiers, guisarmiers/vougiers,
   gros valets, yeomen de la garde, couleuvriniers anglais.

## Prochaine étape

Écrire l'ADR de l'étape 1 après lecture de `skinned.wesl` (bloc variantes), de
`battle_skinned_figures.py` (`FIGURES`, masques) et de `looks.rs`.
