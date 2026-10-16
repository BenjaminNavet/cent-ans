# Lot parch (migration Bevy) : décor de la vue parchemin

Dossier : `client/src/campaign/parchment/`. Port de `parchment_decor.gd`, `parchment_overlay.gd`,
`parchment_glyphs.gd`, `parchment_sea.gdshaderinc` (+ papier/cadre de `parchment_common`).

## État (lot terminé)
- `decor.rs` : placement des roses des vents, navires, serpents, baleines, sirènes (raster de
  distance à la côte, hachage déterministe comme Godot).
- `sea.rs` + `sea.wesl` : plan de mer portulan (lignes de rhumb, rose peinte, papier, cadre et
  marges brûlées au-dessus de la mer), poids = `ZoomView.strategic`.
- `glyphs.rs` : vignettes de ville, navire, serpent, baleine, fanion peints en tiny-skia (×3, mips).
- `realms.rs` : noms de royaume en arc (plus grand bloc connexe par propriétaire, aire ≥ 9000,
  axe ±28°), titre complet en capitales comme Godot (« ROYAUME DE FRANCE »).
- `overlay.rs` : couche UI 2D (ornements de mer, villes + fanions propriétaire/occupant, noms de
  royaume, noms de province en italique, jetons d'armée avec anneau, pulsation de sélection,
  écu et effectif « N NNN »).
- Dump : section `campaign_parchment`.

## Écarts à Godot (volontaires)
- Les lettres des noms en arc s'inclinent avec l'arc (Godot les inclinait à l'envers).
- Déclutter des noms de royaume : un nom dont la boîte touche celle d'un royaume plus vaste ou
  sort de la vue est omis (Godot les superposait).
- Contour du texte approché par `TextShadow`.

## Restes (hors lot ou non portés)
- Frontières épaisses de royaume et hachures d'occupation (FR1, `faction_borders.gdshaderinc`)
  et papier/hachures du parchemin sur terre (`terrain_parchment.gdshader`) : shader de terrain,
  lot campvis/terrain.
- Noms de région de la vue moyenne (`region_labels.gd`) : couche de vue moyenne, s'efface quand
  le parchemin prend le relais ; non porté ici.
- Météo sur parchemin, jetons fantômes, badges de posture des jetons (« ⚔ », « » »), légende
  (lot HUD), estompage des plaques 3D d'armée (lot armies).
- Le cadre n'est dessiné que sur la mer (la terre le cache).

## parch2 (vague 8) : météo, fantômes, badges de posture
- Constat : frontières épaisses + hachures d'occupation (`provinces/borders.rs`, `ViewScale` parchemin),
  noms de région de vue moyenne (`provinces/labels.rs`, jusqu'à 1250) et légende (`hud/help.rs`, bouton
  minicarte) étaient déjà portés ; cadre seulement sur la mer (`sea.wesl`) aussi. Rien refait.
- `parchment/extras.rs` (nouveau) : météo (un nuage peint par grande cellule, `clouds.parchment_icon_cell_px`,
  glyphes pluie/neige/orage/brouillard dans `glyphs.rs`), `ArmyMemory` + jetons fantômes (armées hostiles
  perdues de vue, `stance_cues.json` `fog_memory`, oubli si détruite sous les yeux du joueur), `TokenCues`
  (posture par armée, icônes `hud_stance_*` du joueur).
- `overlay.rs` : le jeton gagne « ⚔ » (ennemi), « » » (en marche) et la pastille de posture (disque parchemin
  cerclé d'or + icône d'encre).
- Écarts : cercle « ami » de l'anneau (alliés) non calculé (le client ne connaît que hostile / joueur) ;
  fantômes dessinés au-dessus des jetons ; texte d'effectif centré approximativement (largeur estimée).
- Restes : estompage des plaques 3D d'armée au parchemin (lot armies) ; entrées de légende propres aux fantômes.
