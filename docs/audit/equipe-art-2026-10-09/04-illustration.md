# Illustrateur — état des lieux (09/10, lecture seule, 5 images regardées)

## 1. État actuel
| Famille | Données | Images | Couverture |
|---|---|---|---|
| Portraits historiques `portraits/chr_*` | 253 | 253 | 100 % |
| Portraits vieillis `portraits/aged/` | 253 | 153 | 60 % |
| Archétypes (ADR 0063) | — | 202 | complète |
| Événements | 153 | 153 | 100 % |
| Factions `fac_*` | 177 | 177 | 100 % |
| Techs / unités / bâtiments / rencontres | 45/39/30/12 | idem | 100 % |
| Codex | 540 | 233 propres | 43 % propres, 88 % avec repli |
| Blasons factions + maisons | 177 + 166 | idem | 100 % |
| Plaques `art/` | — | 8 biomes, saisons, 12 fins, 23 chargements, 18 vignettes | — |

Codex : 243 fiches empruntent l'image de leur entité (repli `ENTITY_ART` de `codex_window.gd`) ; **64 fiches nature sans image** (animaux, arbres, roches, oiseaux).
Provenance : `dn_catalog_illus.json` / `dn_illus_generation.json` (308 images, `endpoint` = « ? » partout) ; factions en local Z-Image (ADR 0190) ; ancre prévue pour NB2 (ADR 0135).

## 2. Forces
- Couverture complète du contenu courant ; repli en cascade ; archétypes déterministes.
- Prompt de style unifié avec interdits ; génération locale gratuite (~1 min/image).
- Vue parchemin réussie (Atlas catalan, FA6).

## 3. Faiblesses
- Résolutions mélangées : illustrations 375 en 640×360 / 161 en 1024×576 ; portraits 245 en 256 px / 8 en 512 ; événements 768×432.
- Cadres incohérents (filet doré peint sur certaines, pas d'autres).
- **Héraldique inventée** : `fac_thomond` (lys, lion), `chr_otto_le_doux` (léopards anglais sur fleurdelisé). Connu, non corrigé.
- Écart de style entre archétypes (peinture semi-réaliste) et portraits historiques (miniature).
- Anachronismes (`evt_feu_de_grange` : grange américaine).
- 26 descriptions de scène sur 153 événements (`event_scenes.json`).
- Pas de key art ni d'écran titre peint (`boot_splash.png` = aplat brun).
- Le Codex grossit (vague H8) → nouveaux trous.

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| 1 | Vrais blasons dans miniatures et portraits (masque + collage, ou img2img) | Fort | M | 0 | héraldique, outils |
| 2 | 100 portraits vieillis manquants | Fort | M (~100 min GPU) | 0 | pipeline portraits |
| 3 | 64 fiches nature (bestiaire/herbier), remplacer icônes et portraits agrandis | Moyen | M | 0 | Codex H8 |
| 4 | Harmoniser formats (1024×576, portraits 512) et règle de cadre unique | Moyen | L | 0 | UI, DA |
| 5 | Archétypes réalignés sur les portraits historiques | Moyen | M | 0 | DA |
| 6 | Descriptions des 127 événements restants, régénérer les plus génériques | Moyen | M | 0 | historien |
| 7 | Key art + écran titre + vrai splash | Fort (1re impression) | S-M | 0 | UI, marketing |
| 8 | Provenance réelle + contrôle auto (fiche sans image, résolution) | Anti-dérive | S | 0 | tests |
| 9 | Parchemin : monstres marins variés | Faible | S | 0 | cartographe |

Solde fal épuisé : 1-6 faisables en local (verrou `~/dev/cent-ans-raw/dn/gpu.lock`).
