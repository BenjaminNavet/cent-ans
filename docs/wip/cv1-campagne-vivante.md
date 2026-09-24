# CV1 — Campagne vivante (partie 1)

Branche : `worktree-agent-ae5263563bc920e75` (partie de `main` `93d466c`). Rendu seulement :
aucune règle, l'information vient du pont (`get_date_label`, `get_province_state`, `settlements()`,
`settlement_detail`).

## Architecture

- `game/scripts/map/campaign_life.gd` (`CampaignLife`) : nœud unique branché dans `campaign_map.gd`
  (création, `refresh(sim)`, `update_view(distance)`), lit ses propres options de ligne de commande
  (`--season=spring|summer|autumn|winter`, `--no-life`, `--devastate=<province>:<0-100>`).
- `game/scripts/map/season_visuals.gd` : saison courante → poids `campaign_season` (paramètre de
  shader global, `project.godot` § `shader_globals`), transition douce entre deux tours.
- `game/shaders/campaign_life.gdshaderinc` : fonctions saisonnières, terroirs, dévastation, ombres
  de nuages, incluses par `terrain.gdshader` (ajouts localisés, crochets d'une ligne).
- `game/scripts/map/terroir_mask.gd` : texture 1024² (R cultures, G vignes, B brûlis, A pâtures)
  peinte autour des colonies selon population et dévastation.
- `game/scripts/map/settlement_growth.gd` : niveau visuel (village → bourg → ville → cité) et
  ajouts (faubourgs, château Kenney, moulins).
- `game/scripts/map/life_effects.gd` : fumées (cheminées, incendies), oiseaux, bateaux.

## État

- [x] 0. Squelette
- [x] 1. Saisons visibles : terrain (parcelles par saison, prés, forêts lointaines, vigne, neige
      d'altitude et de plaine selon nord/est), feuillage 3D (roux/or à l'automne, nus et givrés
      l'hiver, résineux gardés), ombres de nuages ; transition 2,5 s ; `--season=`
- [ ] 2. Terroirs autour des colonies, fumées de cheminée
- [ ] 3. Colonies qui grandissent
- [ ] 4. Dévastation visible
- [ ] 5. Vie ambiante
- [ ] Mesures FPS avant/après, captures par saison

## Mesures de référence (avant, `main` `93d466c`, machine chargée, charge ≈ 22)

| Vue | FPS | Primitives / appels |
|---|---|---|
| Beaune proche (d = 45) | 144,9 | 10,3 M / 861 |
| Paris moyen (d = 400) | 87,5 (9,3 au 1er essai) | 1,7 M / 562 |
| France (d = 1 200) | 123,1 | 0,27 M / 465 |

## Prochaine étape

Lot 2 : terroirs (masque `TerroirMask`) et fumées de cheminée.
