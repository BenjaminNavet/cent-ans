# H12 — Codex (héraldique, calendrier, vie quotidienne) + événements pédagogiques

## État
- [x] A. 27 fiches Codex écrites
- [x] B. 14 événements pédagogiques écrits
- [ ] Tests : pytest complet, cargo test -p data-model (en cours)

## Fiches
- Héraldique (10) : cdx_blason, cdx_emaux, cdx_meubles_heraldiques, cdx_brisures, cdx_herauts, cdx_armoriaux, cdx_armes_de_france, cdx_armes_d_angleterre, cdx_armes_de_bourgogne, cdx_scrope_grosvenor
- Calendrier (6) : cdx_travaux_des_mois, cdx_heures_canoniales, cdx_fetes_chretiennes, cdx_style_de_paques, cdx_calendrier_julien, cdx_horloges_mecaniques
- Vie quotidienne (10) : cdx_vetement_medieval, cdx_lois_somptuaires, cdx_etuves, cdx_echecs, cdx_des_et_hasard, cdx_jeu_de_paume, cdx_cartes_a_jouer, cdx_fabliaux, cdx_maison_mobilier, cdx_eclairage
- Médecine (1) : cdx_feu_de_saint_antoine
- Les fiches d'armes n'ont pas d'`entity` : `fac_france`, `fac_england`, `fac_burgundy` sont déjà portées par cdx_valois, cdx_plantagenets, cdx_bourgogne (codex_store.gd garde une fiche par entité). Liens ajoutés en `see_also` dans ces trois fiches.

## Événements
evt_marchand_venitien, evt_careme_rigoureux, evt_gabelle_du_sel (historique 1341-1344), evt_pain_de_feves, evt_banquet_princier, evt_crieur_de_vin, evt_medecin_de_montpellier, evt_theriaque_frelatee, evt_fondation_leproserie, evt_feu_saint_antoine, evt_saignee_du_roi, evt_monnaie_rognee, evt_traite_des_monnaies (historique 1356-1360), evt_proces_d_armes

## Prochaine étape
Vérifier les tests, rapport final.
