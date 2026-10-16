# bm-court (lot Bevy "court")

Fait (court + court2) : `game-api::court` (fiche, arbre de compétences, famille, cour, conseil, actes royaux, capitaines, chevalerie, ordres, rançons par versements / parole / termes / exécution, contexte de portrait, conseils par charge), `client/src/campaign/panels/court/` :
- `portrait.rs` : portrait vivant (fixe / variante âgée / archétype par hachage FNV-1a, rang, marques, cadre par rang) d'après `data/portraits/archetypes.json`.
- `family_layout.rs` : arbre familial avec traits de liaison (couple double trait, descente, barre, arc de fratrie).
- `skill_layout.rs` : arbre de compétences en bandes/colonnes avec liens prérequis (dorés si appris).
- `view.rs` : suite en vignettes (retinue_row), section Rançon (comptant, 2 à 6 échéances, parole, termes, exécution avec confirmation), conseils (top 3 + par charge, clic = armée/province/écran), portrait du titulaire.
Dump `court`, hook `CENT_ANS_OPEN_COURT`.

Reste : salle 3D du conseil (`council_room.gd`), indice `NextHint` parmi les conseils, miroir/désaturation réelle du portrait (teinte multiplicative faute de shader `portrait_marks`), tirets de l'arc de fratrie (trait plein), zoom/glisser de l'arbre familial, icônes de compétence par nœud.

## campui7 (vague 8)
Fait : `NextHint` parmi les conseils (`hint_alert`, id `hint:<id>`, un clic relaie `hud:hint_go` à la carte du HUD) ; shader `portrait_marks.wesl` (`PortraitMarksMaterial`, `UiMaterial`) : grisaille du défunt, pâleur du malade, ombre du captif, miroir en UV (remplace la teinte) ; zoom de l’arbre familial (`+`/`−`/Ctrl + molette, 45-160 %, `UiTransform`) et défilement au glisser, décalage conservé d’une reconstruction à l’autre.
Reste : salle 3D du conseil, tirets de l’arc de fratrie, icônes de compétence par nœud, zoom de l’arbre de compétences, « recentrer sur le souverain » (⌂ remet zoom 100 % et début de l’arbre), clic droit = recentrer, pincement (magnify).
