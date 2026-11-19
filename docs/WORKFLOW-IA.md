# Workflow : de l'image à l'illustration avec une IA

C'est l'usage premier du projet. On fournit à un assistant IA capable de lire des images et de modifier des fichiers (testé avec Claude Code) :
1. une ou plusieurs **images de référence** de l'objet ;
2. une **description** (ce qu'il faut garder, simplifier, ignorer) ;
3. le **comportement** voulu (quelles pièces bougent, comment).

L'IA interprète l'image et **écrit un modèle** (`web/modeles/<nom>.json`, voir `docs/FORMAT-MODELE.md`). Le moteur le dessine en contours, l'anime et l'exporte. **Le moteur ne reconstruit pas un objet 3D à partir d'une photo** : c'est l'IA qui décrit l'objet avec des boîtes, des cylindres et des prismes, donc la ressemblance est approximative et volontairement stylisée.

## Règle : un briefing complet avant de construire
Pour éviter les aller-retours, l'IA ne commence pas la modélisation avec seulement des images. Elle pose d'abord les questions ci-dessous (celles dont la réponse n'est pas déjà donnée), fait un récapitulatif, et attend la validation avant de construire.

1. **Références** : images fournies ; ce qu'il faut modéliser ; ce qu'il faut ignorer (câbles, prises, arrière…) ; niveau de détail (très simplifié, moyen, détaillé).
2. **Dimensions** : taille réelle de l'objet ou proportions ; sinon l'IA les estime et le dit.
3. **Rendu** : contours (défaut : traits gris clair `#c8cbd0` sur fond sombre `#0d0e10`) ou couleurs d'origine ; couleurs du fond, des contours et de la teinte des pièces animées (défaut `#ff6363`) ; épaisseur du trait (défaut 1,3 px).
4. **Vue** : rotation horizontale (`az`) et verticale (`el`) en degrés ; focale en mm (courte = perspective marquée, 50 = neutre, longue = écrasée) ; zoom (cadrage serré, normal, large) ; vue fixe ou caméra réglable.
5. **Animation** : pour chaque pièce mobile, le type de mouvement (rotation ou translation), l'axe, l'amplitude et ce qu'elle représente ; mouvement automatique (période en secondes, vitesse, décalage entre pièces) et/ou curseur manuel.
6. **Sortie** : page HTML animée, SVG figé ou JSON ; usage prévu ; format de l'image ; nom du fichier.

## Ce que le moteur sait faire (l'IA ne propose que cela)
- Pièces : boîte (avec chanfrein), cylindre (axe x, y ou z), prisme à contour convexe. Pas de formes courbes libres, pas de transparence, pas de dégradés.
- Décors à plat sur les faces : rectangle, cercle, polygone, texte, grille de trous ronds.
- Mouvement : une commande déplace ou tourne une ou plusieurs pièces ; la démonstration la fait osciller entre son minimum et son maximum. Pas de déclenchement au scroll ni au clic dans la page exportée.

## Construire et vérifier
1. Écrire le modèle, le charger dans `web/index.html` (serveur local : `python3 serve.py`).
2. Vérifier à 390 px et à 1440 px de large : capture, console sans erreur, mouvement réel (curseurs et démonstration), page exportée ouverte à part.
3. Dire honnêtement les écarts avec la référence et ce qui n'a pas été vérifié.

## Droits
Ne pas décalquer une photo de produit existant, ni reprendre son nom ou son logo. Modéliser des formes génériques ou des objets dont on détient les droits.
