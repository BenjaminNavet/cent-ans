# 0741 — SMAA sur la carte de campagne au lieu du MSAA

Date : 2026-10-23 · Chantier camp60 (60 i/s en carte de campagne)

## Contexte

En Ultra, la carte rendait en MSAA 2x (`map_msaa`). Mesures sur M4 Pro, plein écran Retina
3456×2234, mise à l'échelle automatique (ADR 0740, ≈ 1080p rendus), panoramique à 30 u. :

| variante | p50 (ms) |
|---|---|
| Ultra MSAA 2x | 55-62 |
| Ultra MSAA off | 45,6 |
| terrain plat (couleur seule) MSAA 2x | 21,4 |
| terrain plat MSAA off | 20,0 |

Avec un shader plat, le MSAA coûte 1,4 ms ; avec le vrai shader de terrain, il en coûte environ 17.
Ce shader procédural est très lourd (2 400 lignes : champs, parcelles, voronoï, couches texturées).
En MSAA, chaque pixel à cheval sur un bord de triangle est ombré une fois par triangle. Le
maillage du relief est fin (un sommet tous les 6 px), donc une grande part de l'écran est sur
un bord.

## Décision

Nouvelle clé de préréglage `map_smaa` (`data/fx/render_quality.json`, schéma à jour) :
- Moyen, Élevé et Ultra : `map_smaa: true`. Ultra passe à `map_msaa: "off"`.
- Le système `apply_campaign_smaa` (`campaign/camera/mod.rs`) pose ou retire `Smaa` sur la
  `CampaignCamera`.
- Si le joueur force un niveau de MSAA dans les Réglages, son choix l'emporte et le SMAA n'est
  pas posé (`smaa_campaign = map_smaa && msaa auto`), comme pour le SSAO en bataille.

## Conséquences

- Ultra au réglage du joueur : 55-62 → 45,8 ms ; le SMAA lui-même ne se mesure pas (< 0,5 ms).
- Moyen et Élevé gagnent un anticrénelage qu'ils n'avaient pas, presque gratuitement.
- Le SMAA lisse moins bien que le MSAA les arêtes très fines (fils, lances). Sur la carte, vue
  de haut, ce sont surtout les silhouettes du relief et des bâtiments, que le SMAA traite bien.
