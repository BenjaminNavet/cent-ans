# Lot overlays (correctifs post-playtest 17/10)

Branche `fix/overlays`. ADR 0446.

## Fait (code, tests unitaires verts)
- 1/2/3 : avis et sous-titres du conseiller sous les panneaux et modales (`UiTier::Toast` = 5) ; tutoriel, guide féodal, offre de mission attendent la file de modales ; guide féodal attend le tutoriel ; conseiller muet (réplique mise en file) pendant modale / pause / carte du tutoriel.
- 4 : `KeyMap::set_suspended` pendant la Pause (`menu/pause.rs::suspend_shortcuts`).
- 5 : `modal_escape` consomme l'arête Échap, `pause_escape` passe après.
- 6 : `court::shortcuts` branche J (Conseil) et C (fiche du souverain).
- 7 : `{key:ACTION}` (`KeyHints::expand`) dans `next_hints.json` et `steps.json`.
- 8 : règle du core = réserve plafonnée (`research_reserve_turns`), puis perdu ; textes alignés.

## Vérifié (dump + 5 captures, bac sb-overlays)
- 4 : pause ouverte, F3/F4 -> `tech.open` et `diplomacy.open` faux ; après reprise F3 ouvre.
- 5 : F1 -> modale `hud:shortcuts`, Échap -> modale fermée, `menu.paused` faux.
- 6 : J -> court.screen Council (re-J ferme), C -> Sheet.
- 7/8 : carte « Conseil » lit « (F4) » ; texte aligné sur la réserve plafonnée.
- 2 : fin de saison : modale sur l'avis ; ensuite offre de mission seule, guide féodal après « Plus tard ».
- Tests : client 1313 verts, clippy propre, smoke 10/10.

## Non reproduit / limites
- Baseline non rejouée (build de départ mêlé aux premières retouches) : défauts 1-5 établis par lecture du code puis preuves après correctif.
- Bulle du conseiller en bataille (masque « Commencer ») : non reproduite sur une bataille personnalisée (bulle dans l'emplacement Journal du HUD, étage HUD). Le cas avis/toast est couvert par l'étage 5.
- Panneau d'armée resté ouvert après fin de saison : état du joueur, non touché.
- Réplique d'ouverture du conseiller rejouée au chargement (défaut ux n°2) : hors lot.

## Hors lot
- Textes `data/ui/tooltips.json` (hud_court / hud_technologies / hud_diplomacy) avec touches C/T/P codées en dur : non utilisés par le client (les infobulles de la barre lisent la keymap).
