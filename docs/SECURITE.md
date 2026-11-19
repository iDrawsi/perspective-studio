# Sécurité

## Dépendances
Le projet n'a **aucune dépendance externe** : `Cargo.lock` ne contient que le moteur lui-même, il n'y a ni npm ni bibliothèque JavaScript, et la liaison avec WebAssembly est écrite à la main (`web/glue.js`, fonctions `extern "C"` dans `crates/core/src/lib.rs`). Ajouter une dépendance demande une étude préalable (qui la maintient, que fait-elle à la compilation, quelle version exacte, `Cargo.lock` commité).

## Code
- Le moteur ne contient aucun bloc `unsafe` ; seul l'attribut `#[unsafe(no_mangle)]` marque les fonctions exportées vers le navigateur.
- Le modèle arrive dans le moteur par de petits appels numériques ; il n'y a pas d'analyse de texte côté Rust. Les valeurs non finies ou hors bornes donnent un rendu vide ou une erreur, jamais un plantage.
- Côté page, tout fichier de modèle chargé est validé et borné (`lireModele`, `web/modele.js`) : types connus seulement, nombres limités, couleurs au format `#rrggbb`, nombre de pièces et de décors plafonné, taille du fichier limitée. Les textes sont échappés avant d'entrer dans le SVG.
- La page HTML exportée n'appelle aucune adresse réseau. Le modèle y est inséré sans le caractère `<`, pour qu'aucune balise ne puisse fermer le script.

## Serveur local
`serve.py` écoute uniquement sur `127.0.0.1`, ne sert que le dossier `web/` et demande au navigateur de ne rien garder en cache. Ne pas le lancer depuis un dossier contenant des données personnelles ni l'exposer sur un réseau.

## Signaler un problème
Ouvrir une « issue » sur le dépôt, sans y joindre de données personnelles.
