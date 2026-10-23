# 0740 — Mise à l'échelle du monde 3D (Bevy)

Statut : accepté (23/10/2026, chantier camp60)

## Contexte
Le réglage « Mise à l'échelle » (ADR 0123, Godot : MetalFX spatial) était sans effet dans le client Bevy : le monde était
rendu à la résolution native. Sur l'écran du joueur (Retina 3456×2234 en plein écran, 7,7 Mpx, qualité Ultra), la carte de
campagne tournait à 10 fps (panoramique d = 30 : p50 101 ms). Le coût est surtout par pixel (shader du sol : 18 ms sur 37 à 1280×800).

## Décision
- Une caméra 3D marquée `ScaledWorldCamera` rend dans une image à `échelle × cible finale` (`settings/render_scale.rs`),
  affichée en plein écran sous l'interface par la caméra d'interface (filtrage bilinéaire, nœud `WorldUpscale`, non survolable).
- L'image porte le `scale_factor` correspondant : la taille *logique* de la caméra reste celle de la fenêtre, donc les rayons du
  curseur, `world_to_viewport` et les ancrages d'interface ne changent pas.
- Échelle lue dans `data/fx/render_quality.json` § `upscale` : « Automatique » garde le nombre de pixels de la référence 1080p
  (au moins `min_scale`) ; Qualité 0,75 ; Performance 0,5 ; Désactivée 1. `CENT_ANS_RENDER_SCALE` force une valeur (bancs).
- Interface et texte restent à la résolution native.

## Conséquences
- Campagne, plein écran du joueur : p50 101 → 55 ms (première mesure).
- Image du monde plus douce qu'en natif ; un suréchantillonnage plus net (bicubique, accentuation) peut remplacer le bilinéaire.
- Seule la caméra de campagne est marquée pour l'instant ; la bataille peut l'être de la même façon.
