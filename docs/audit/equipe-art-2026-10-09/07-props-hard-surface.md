# Prop / hard-surface artist — état des lieux (09/10, lecture seule)

Constat : beaucoup d'objets produits, mais les écrans de siège montrent encore des boîtes. Il faut **brancher l'existant** plus que produire.

## 1. État actuel
- **Engins** (`game/assets/models/siege/`) : `ga3_trebuchet`, `ga3_ram` (+ `_lod`, articulés via `ga3_rigs.json`) ; `mangonel`, `bombard`, `siege_tower` + trébuchet/bélier procéduraux (`siege_engines.py`, 360-800 tris). Pivots dans `siege/manifest.json`, animation `siege_engines_fx.gd`, `siege_crew_fx.gd` ; échelles `_make_ladder`.
- **Décor GA3** (`props_ga/`) : 9 objets × 3 LOD (maison, église, moulin, chariot, tente, palissade, puits, trébuchet, bélier) ; `battle_decor.gd`, `battle_village.gd`, `ga3_kit.gd`.
- **Carte vivante** (`folk/`) : charrettes, étal, charrue, torche, procession, bûcher, échafaud (procéduraux CC0, `folk_props.py`).
- **Campagne** : `fleet/cog`, `nef`, `bivouac` (`campaign_fleet.py`) ; `siege_camp.glb`, `worksite_1` = placeholders ; enseignes, moulins, ports, puits.
- **Paquet DN** (`models/dn/`, 1,1 Go, hors git, ADR 0212) : `dn/siege` 19 (beffroi, trébuchet, bombardes 1380/1450, canon 1340, ribaudequin, espringale, mantelets, épaves) ; `dn/ships` 37 ; `dn/props` ~165 (tentes, chariots, décombres, fortifications de campagne, piloris, gibets, fontaines). Catalogues `data/art/dn_catalog_*.json`.
- **Bannières** : quads + shader (`map_banner`, `battle_standard_flag`), hampe procédurale.

## 2. Forces
- Engins fonctionnels avec pivots et animations, règles associées (ADR 0176).
- Chaîne gratuite et documentée (ADR 0210), LOD systématiques, manifestes.
- Datation riche (canons 1340/1380/1450).
- Navire choisi selon bassin/culture/année (ADR 0217) avec repli.
- Paquet DN relu (13 objets refaits).

## 3. Faiblesses
1. **Rien de DN n'est utilisé en bataille** : 19 engins + ~95 entrées `battle_extra` dorment.
2. Tour, bombarde, mangonneau = boîtes (captures `sg3_avignon_beffroi_pousse.png`, `sg3_bruges_bombarde_charge.png`) ; tours trop grandes.
3. Aucun engin sur la carte de campagne ; camp et bivouac placeholders.
4. Armes procédurales, armes lâchées en boîtes.
5. Paquet DN non publié → inactif ailleurs.
6. Pas de navires en bataille 3D (voulu) ; cog/nef procéduraux sans DN.
7. Modèles TRELLIS mono-surface : pas de surface `Banner` recolorable.

## 4. Améliorations
| # | Action | Impact | Effort | Dépend de |
|---|---|---|---|---|
| 1 | Brancher `dn/siege` en bataille via un registre (type `dn_water_models.json`), bombarde par année, repli procédural, re-découpe des pièces articulées | Très fort | M | tech art, bataille, cœur |
| 2 | Corriger l'échelle des tours (cœur) | Fort | S | cœur |
| 3 | Publier le paquet DN (fin ADR 0212) | Préalable | S | joueur, release |
| 4 | Camp/bivouac DN en campagne ; engin devant la ville selon l'avancement du siège | Fort | M | carte, UI siège |
| 5 | Décor de bataille DN (tentes par culture, laager, forge, décombres de brèche) | Moyen-fort | M | environnement, VFX |
| 6 | Armes rigides générées (≤ 800 tris), armes lâchées réelles | Moyen | M | character, tech art |
| 7 | Hampes/fleurons sculptés, surface `Banner` séparée | Moyen | S-M | tech art, héraldique |
| 8 | Remplacer les props FK par leurs équivalents DN | Faible-moyen | S | FolkModels |
| 9 | Navires DN pour le naval 3D (seulement si relancé — le joueur ne le veut pas) | — | L | design |
| 10 | Mobilier de la ville assiégée en DN | Moyen | S-M | cœur, level |

Coût : gratuit. Pour 1, 4, 5 : budget de tris en MultiMesh, A/B perf, ≤ 3 captures.
