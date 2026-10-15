# bm-borders : bandes de frontières en pixels écran (lot borders2)

Portage de `game/shaders/faction_borders.gdshaderinc` + `game/scripts/map/faction_borders.gd` + `data/map/faction_borders.json`.

## Cause du défaut
Le ruban Bevy avait une largeur FIXE en unités carte (`border_world_per_px = 3`) : épaisseur à l'écran proportionnelle à 1/distance (186 px de large à 60 !, 0,84 px pour la ligne de province à 2600) et pas de dégradé ni de contour. Godot peint des bandes de largeur constante en pixels écran (+ `far_width_scale` 1,25 entre 0,4 x 900 et 900).

## Ce qui est porté
- `border.wesl` : le sommet est déplacé de `offset_px x (unités monde par pixel écran)` (profondeur et hauteur de viewport), donc largeurs en pixels à toute distance. Attributs : UV0 = direction vers l'intérieur, UV1 = (décalage px, type de bande).
- Halo intérieur : dégradé `(1 - ds/glow)^1,35` depuis la ligne (fragment), alpha 0,62 x facteur « repos » ; contour sombre (`outline_px` 2,8 / alpha 0,7) sous le cœur ; couches dans l'ordre Godot : halo, hachures, contour, cœur, lignes de province.
- Hachures d'occupation : bande de `band_px` (10) x échelle, chute douce 0,6..1, motif diagonal à deux octaves fondues, période 7 px écran, duty 0,5 (fragment, comme `fr1_hatch_at`). Frontière de royaume = propriétaire OU contrôleur différent (comme Godot).
- Ligne de province : +-0,7 px de chaque côté (1,4 px au total, avant 0,7 u/2), luminosité 0,9 sur la couleur (pas sur l'alpha).
- Style « repos » (TB2) : largeurs x 0,6, alpha x 0,6, halo x 0,25, désaturation 0,45 et luminosité 0,8 (en linéaire, `rest_look`) ; plein régime : survol/sélection, ennemis (ADR 0155, rouge réservé aux ennemis), mode Diplomatie, vue parchemin (`parchment_focus`, clé de reconstruction).
- Joueur : largeur x 1,25, halo x 1,35, luminosité x 1,25 (halo dans le maillage séparé qui respire). Parchemin : largeur x 1,7, halo x 0,6, alpha x 1,15 (poids `ZoomView.strategic`).
- Couleurs de relation : déjà portées (`modes::border_color`), inchangées.

## Largeurs effectives (px écran, deux côtés de la ligne pour cœur / contour / ligne de province ; halo et hachures d'un côté ; plein régime, hors joueur)
Section de dump `campaign_provinces.border_widths` (`focused`, `rest`, `player_focused`, `view_scale`, `camera_distance`). Colonne Bevy « après » = valeurs du dump (`out/borders2.json`, vérifiées identiques à la formule Godot) ; test `widths_match_the_godot_formula`.

| distance | Godot réel (formule) cœur / contour / halo / ligne prov. / hachures | Bevy avant (3 u/px fixe) cœur / ligne prov. | Bevy après (dump) |
| 2600 | 4.50 / 7.00 / 16.25 / 1.75 / 12.50 | 4.3 / 0.84 | 4.50 / 7.00 / 16.25 / 1.75 / 12.50 |
| 1200 | 4.50 / 7.00 / 16.25 / 1.75 / 12.50 | 9.3 / 1.82 | 4.50 / 7.00 / 16.25 / 1.75 / 12.50 |
| 600 | 3.98 / 6.18 / 14.36 / 1.55 / 11.04 | 18.7 / 3.63 | 3.98 / 6.18 / 14.36 / 1.55 / 11.04 |
| 200 | 3.60 / 5.60 / 13.00 / 1.40 / 10.00 | 56.0 / 10.89 | 3.60 / 5.60 / 13.00 / 1.40 / 10.00 |
| 60 | 3.60 / 5.60 / 13.00 / 1.40 / 10.00 | 186.7 / 36.31 | 3.60 / 5.60 / 13.00 / 1.40 / 10.00 |

Au repos (x 0,6) : cœur 2,16 à 3,6 -> 2,70 ; ligne de province 0,84 ; halo 7,8 -> 9,75 ; hachures 6 -> 7,5.

## Écarts et restes
- Godot lisse le bord des bandes par smoothstep +-0,6 px ; ici bord franc, anti-crénelage MSAA.
- L'altitude des sommets est échantillonnée au point d'arête (le décalage de quelques pixels n'est pas rééchantillonné) : pente raide + halo large = léger écart de drapé.
- Le halo du joueur est dans un maillage séparé (respiration) : il peut se superposer au cœur des arêtes voisines (même teinte, sans effet visible attendu).
- Le niveau de qualité Basse (halo / hachures coupés) n'est pas encore branché (`BorderStyle.glow/hatch` toujours vrais).
- Pas de contrôle visuel (règle : aucune capture) : à juger par le joueur.
