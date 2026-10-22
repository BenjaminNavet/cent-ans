# Lot M — remplaçants musicaux d'époque (1337-1453)

Suite de `docs/wip/a2-audio-credits.md` et `docs/wip/b6-docs-audio.md` (point ouvert « Anachronismes »).
Source : `docs/wip/nuit-critiques/audio.md`, constat 2.

## État

- Fait : retrait des pistes anachroniques de leurs playlists : Greensleeves et Gibbons (`campaign_england`),
  Cabezón et Ortiz ×2 (`campaign_iberia`), Gibbons, Cabezón, Ortiz ×2 (`court`), Moulinié (`campaign_burgundy`
  et `court`). Fichiers conservés sur disque, rien supprimé.
- Fait : remplaçants d'époque déjà sur disque et créditables (sans nouveau téléchargement) :
  - `campaign_england` : Bryd one brere (v. 1300, Ars nova, enregistrement 1963) et Sumer is icumen in (XIIIe s.)
    promus en `primary` ; Tomsińska conservé (voir point ouvert 1).
  - `campaign_burgundy` : Dufay « Ave Regina » (CC0) et Binchois « Adieu m'amour » (PD) en `primary`,
    remplacent Moulinié.
  - `campaign_iberia` : Cantigas de Santa María (XIIIe s.) et Folía d'Ahigal en `primary` (voir point ouvert 2).
  - `court` : Dufay « Se la face ay pale » et Hé Robinet (v. 1450) en `primary`, remplacent Gibbons, Cabezón,
    Moulinié, Ortiz.
- Fait : `data/audio/music.json` : gains orphelins supprimés (Cabezón, Gibbons, Greensleeves, Ortiz ×2) ;
  `data/audio/credits.json` : 5 entrées retirées, 51 pistes tierces créditées (test « jouée = créditée » vert).
- Tests : `uv run --project tools pytest -q -k "schema or music or audio or credits or era_music"` : 1822 passés.

## Non fait, à trancher

1. **`war` : Moulinié reste en `primary`** (seule piste primary). Le remplaçant naturel est l'Agincourt Carol
   (`wikimedia/agincourt_carol_deo_gracias.mp3`, PD, non référencé aujourd'hui), mais
   `tools/tests/test_rx_audio.py::test_menu_war_battle_share_no_piece` interdit « agincourt_carol » en `primary`
   sur toutes les playlists. Choix à faire : (a) assouplir ce test (décision de l'auteur du test, à confirmer) ;
   (b) garder une autre piste de guerre. Ne pas le placer en repli pour contourner le test. Agincourt n'est donc
   ajouté à aucune playlist ni à `credits.json`.
2. **`campaign_islamic` : inchangé.** Istikhbar Mezmoum (1929) et Hüseyni saz semaisi (avant 1939) restent en
   `primary`. Aucune piste d'époque (avant 1453) sur disque pour l'islamique ; les maqams de 1931 sont hors période.
   Manque : pistes libres d'oud ou de qanun médiévaux (XIIe-XVe s.) à chercher sur Wikimedia / Musopen / IMSLP.
3. **Tomsińska (luth, XVIe s.)** reste en `primary` dans `campaign_england`, `campaign_burgundy`, `campaign_iberia`
   et `campaign_france`. C'est le même cas que Le Roy (1551), retiré par A2 ; hors liste demandée, donc non touché.
   Décision à prendre : le retirer aussi, ce qui exige un second remplaçant par région.
4. **Folía d'Ahigal** (1988, tradition orale, genre attesté vers la fin du XVe s.) : placé en `primary` ibérique
   faute d'autre piste médiévale sur disque. Constat à confirmer à l'oreille.
5. **Non modifiés** : `CREDITS.md` (racine) mentionne encore les pistes retirées ; `era_music.py` régénère
   `wikimedia/SOURCE.md` depuis ses manifestes et ne sort pas les pistes retirées des playlists (les fichiers
   restent sur disque, donc leurs crédits restent exacts).
6. `campaign_italy` et `campaign_france` n'ont pas été modifiés (non cités par le lot).
