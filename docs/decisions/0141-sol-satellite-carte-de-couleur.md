# 0141 — Sol « satellite » : carte de couleur précalculée

Date : 2026-09-30. Chantier SS. Spec : `docs/superpowers/specs/2026-09-30-ss-sol-satellite-design.md`.

## Contexte

Le joueur trouve la carte de campagne « façon IGN » à toutes les hauteurs. Captures avant (Paris, distances 900 / 300 / 90) : en vue moyenne et proche, les terres ouvertes sont gris-beige pâle, l'ombrage du relief (`shading_relief` 2,1 + `relief_shade` fin) et ses hachures dominent, les parcelles (`fine_parcels`, visibles seulement de près via `fp_near`) ne ressortent pas, les routes sont des traits crème uniformes. Cible : Total War (Attila / Pharaoh), patchwork agricole lisible, routes de terre, lacs avec rives.

Le terrain est déjà la couche la plus chère (~40 ms masqué/affiché, `docs/wip/fps-carte.md`) : pas de procédural supplémentaire de loin.

## Décision

1. **Carte de couleur hors ligne** `cent-ans geo colormap` : albédo RGB 14336×12288 (2 × la carte, ~360 m/texel), mipmaps précalculées, **BC1** en parts zlib, décrite dans `map.json` (`colormap.bc1`) — même chemin que `wetlands_gpu` et `relief_shade.bc5` (ADR 0118). Pas de pyramide en tuiles : une texture unique suffit pour la vue large et moyenne, et reste versionnée (~60–90 Mo compressés).
   Contenu : base régionale (splat, sécheresse, altitude, forêts), mosaïque agricole de blocs de 1–3 km (cultures de la palette) autour des colonies, routes en terre claire à largeur par `type`, rives de lacs (grève, roselière, eau peu profonde), auréoles de villes. Palette et paramètres dans `data/map/colormap_style.yaml` (schéma `data/schemas/colormap_style.schema.json`), graine fixe.
2. **Shader** : la carte de couleur remplace la couleur procédurale en vue large et moyenne ; les parcelles procédurales (crop par parcelle déjà présent) prennent la couleur de leur bloc et restent le détail de près ; textures 2k en luminance seulement ; ombrage du relief abaissé. Repli sur l'ancien chemin si la carte manque.
3. **Routes** : peintes dans la carte de loin ; `road_line` restylé en terre (teinte, largeur par type) et effacé en vue large.
4. **Lacs** : maillage d'eau (`lakes_renderer.gd`, contours extraits hors ligne dans `data/map/lakes.json`) avec `river_water.gdshader`.
5. **fal.ai** seulement pour des textures de détail tuilables (et au plus 2–3 petits objets), après jugement des captures.

## Conséquences

- +60–90 Mo dans le dépôt ; +~117 Mo de VRAM (BC1 avec mipmaps). Une lecture de texture remplace une partie du procédural lointain : coût GPU attendu ≤ actuel, vérifié au banc.
- Toute modification de palette passe par l'outil (re-cuisson ~minutes), pas par le shader.
- La carte de couleur est une saison moyenne ; `campaign_life` continue de moduler saisons et terroirs de près.
