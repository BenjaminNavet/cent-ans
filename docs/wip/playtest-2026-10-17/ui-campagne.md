# Rapport UI campagne (testeur UI designer)

Dossier de travail : `$P/ui-campagne/` (P = scratchpad/playtest). Captures en 1280x800 (VIEW_SIZE fixe, `client/src/agent/mod.rs:34`).
Réglages utilisateur réels utilisés par défaut (ui_size 1.1, text_size 0.9). Fichiers de preuve cités ci-dessous, relatifs à `ui-campagne/`.
Note : la résolution hors-écran est une constante ; seule la combinaison « Taille de l'interface / du texte » (via `CENT_ANS_SETTINGS_PATH`, `set_big.json` ui 1.6 / texte 1.3, `set_small.json` ui 0.7 / texte 0.8) a pu varier.

## MAJEURS

1. **Recrutement : la liste des cartes d'unités est réduite à une fente, le reste déborde du panneau** (échelle par défaut).
   - Repro : `$P/play.sh --state Campaign --faction fac_france --script s2.txt` (clic `hud:open_recruitment`).
   - Observé : la zone `rec:cards` ne fait que quelques px (aucune carte lisible) ; la liste « Bientôt » occupe tout l'espace ; « Panier vide… », « Sceller la levée / Vider » sortent sous le cadre du panneau. Preuve : `o_recruitment.png` ; à l'échelle 0.7 tout rentre (`small_recruitment.png`). Sonde `ui_kit.layout` : `rec:panel` et `rec:cards` en échec (`big_recruitment.json`, `small_recruitment.json`).
   - Attendu : cartes visibles, « Bientôt » repliable/limité, boutons dans le cadre.
2. **Économie : le tableau du budget dépasse le bas du panneau** (`o_economy.png`) : lignes « Entretien… », « Solde de la saison » dessinées sous le cadre, qui s'arrête vers y=540 alors que le contenu va jusqu'à y=600. Sonde : `EconomyPanel` en échec.
3. **Traditions d'armée : le contenu déborde du panneau** (non clippé) et se superpose aux alertes de droite (« Tenue — Serrer les rangs », bouton Choisir sur « Chantier libre »). Repro : `CENT_ANS_OPEN_TRADITIONS=1 $P/play.sh --state Campaign --script one_trad.txt` (`trad.png`). Attendu : zone défilante.
4. **Échelle d'interface « Très grande » (ui 1.6, texte 1.3) : HUD cassé** (`big_hud.png`, `big_diplomacy.png`, `big_economy.png`) : barre du haut tronquée (5 boutons sur 11 visibles, les autres hors écran) ; cadre blanc vide du minimap qui dépasse à droite ; bulle du conseiller recouvre bouton Journal, le texte « Conseil » et le haut des panneaux ; Diplomatie/Économie/Conseil tronqués (titre masqué, contenu coupé en bas). Sondes : `hud:top_bar`, `hud:minimap`, `hud:turn_wait`, `hud:journal`, `hud:hint`, `hud:map_filters_menu` en échec ; `DiplomacyPanel`, `dip:list`, `CourtPanel` en échec aux deux échelles testées.
5. **Sélection de faction : barre d'actions du bas coupée à 1280x800** : « Commencer — France » et « Retour » à moitié hors écran (`m_fs.png`) ; en plus, des éléments du menu principal (« C », « des », cartouche titre) transparaissent au travers des panneaux et du bandeau de date. Esc ne revient pas au menu principal (`m_after_fs_esc.png` identique). Repro : `$P/play.sh --state MainMenu --script menu.txt`.
6. **Colonies/Bâtiments : « Emplacements 9/6 »** sur Paris (Cité) (`pt_s.png`) : occupation supérieure à la capacité. Les pastilles d'emplacement sont minuscules, toutes semblables, avec libellés ambigus (« Bâti », « 1/2 », « Niv. 3 », « 3/4 », « + »). Repro : `CENT_ANS_OPEN_PROVINCE=capital $P/play.sh --state Campaign --script provtabs.txt`.

## MINEURS

7. **Bulle du conseiller (Jean le Bel) : ordre d'empilement incohérent avec les panneaux plein écran** : par-dessus Diplomatie/Conseil/Cour (cache les onglets « Famille » et « Ordres » de la fiche du roi, `court.png`), mais sous Technologies/Chronique. Elle masque aussi la zone Journal/hint à grande échelle.
8. **Lieu de l'armée en minuscules sans accents** : « à paris » (`army.png`), cause : `client/src/campaign/panels/army/view.rs:229` (`s.replace("set_","").replace('_'," ")` sur l'identifiant au lieu du nom localisé). Ex. pour d'autres provinces : « ile de france ».
9. **Montre des hommes d'armes : « entretien 0 livres par saison »** (`roster.png`) alors que le panneau d'armée affiche « entretien 735 ₶ / saison » et le budget « Entretien des armées −4 830 ₶ ». Incohérence de valeur ; aussi « livres » vs symbole ₶ selon les panneaux.
10. **Boutons de fermeture incohérents** : « × » (Diplomatie, Conseil, Économie, Féodal, Province, Faction), « Fermer » (Technologies, Chronique, Recrutement, Objectifs, Traditions, Montre, Armée). Position et taille variables.
11. **Conseiller générique et genré** : Navarre (reine Jeanne II) lit « Monseigneur, … » et le même chroniqueur « Jean le Bel » sert toutes les factions (`small.png`).
12. **Technologies : colonne Rang 4 coupée à droite et dernière ligne coupée en bas** sans indication de défilement visible (`tech.png`). Féodal : dernière ligne de l'arbre coupée à moitié (`feudal.png`).
13. **Main menu** : entrées de menu superposées à la rose des vents (contraste faible sur « Prologue », « Codex »), titre « Cent Ans » dupliqué (logo + cartouche) (`m_main.png`).
14. **Texte minuscule à ui 0.7 / texte 0.8** (`small_*.png`) : captions illisibles (~7 px de haut). Option permise par Réglages.
15. **Réglages > Affichage** : « Plein écran » coché mais « Résolution (fenêtré) » reste modifiable ; la rangée de résolutions passe à la ligne de façon désalignée (`m_set.png`). Esc ferme bien le panneau.

## COSMÉTIQUES
16. Titre « l'ost de … » en minuscule initiale dans les titres de panneau (« l'ost de Philippe VI… », devrait être « L'ost »).
17. Barre du haut : icônes sans libellé, seules les touches (J, F4, F3…) indiquent la fonction ; infobulles non vérifiées (pas de commande de survol en script).
18. Sonde `ui_kit.layout` : 6 échecs au repos dès l'état de départ (`hud:top_bar` marge requise 8/6 vs 12/4 réel, `hud:hint`, `hud:journal`, `hud:map_filters_menu`, `hud:minimap` enfant à (-64,-390), `hud:turn_wait` requise 44/40 vs 17/8) : à vérifier si ce sont de vrais défauts de marge ou des seuils trop stricts.

## Ce qui marche bien
- Échap ferme proprement tous les panneaux testés (Diplomatie, Technologies, Chronique, Féodal, Conseil, Faction, Économie, Recrutement, Objectifs, Codex, Montre) : image revient à la base (diff < 6/255), et ouvre le menu Pause quand rien n'est ouvert. Menu Pause complet (Reprendre, Sauvegarder, Charger, Batailles jouées, Réglages, Aide, Menu principal, Quitter) bien aligné.
- Aucun identifiant brut (`fac_…`, `prov_…`, `bld_…`) visible, aucun NaN ni valeur négative absurde dans les panneaux vus, tout en français. Faction England et petite faction (Navarre) : HUD correct (trésor, revenu, manpower, saison).
- Diplomatie (opinion détaillée, clauses, jauge), Conseil, Fiche du roi, Codex, Objectifs, Province (Ville), Garnison, Armée : lisibles et cohérents au rendu par défaut.

## Non testé / limites
- Résolution hors-écran fixe 1280x800 : pas de test 1920x1080 ni 2560x1440 (seule l'échelle UI a varié).
- Infobulles de survol et chaînes Alt, avis/toasts, siège (`CENT_ANS_STAGE_SIEGE` avec un id deviné n'a rien ouvert), Colonies d'autres provinces, Technologies/Chronique en cours de partie, Sauvegarder/Charger, Codex en détail, Crédits, Prologue, boutons `hud:open_ransoms` / `hud:open_traditions` absents au démarrage (conditionnels, `click_if` sauté).
- Faction `fac_france` testée à fond, `fac_england` et `fac_navarre` en HUD uniquement (pas d'ouverture de panneaux).

## Versions
- Tous les défauts ci-dessus ont été vus sur l'ancien binaire (7e0c4bb12).
- Revérifiés sur le nouveau binaire (3dd37ab5c) : n°1 Recrutement (cartes en fente, boutons hors cadre : `v2_recruitment.png`) et n°2 Économie (tableau hors du cadre : `v2_economy.png`) persistent. n°7 persiste et touche aussi Économie et Recrutement : la bulle du conseiller recouvre les titres « Économie » et « Recrutement — Paris ».
- Non revérifiés : n°3 (Traditions), n°4 (échelle Très grande), n°5 à 6 et mineurs.
