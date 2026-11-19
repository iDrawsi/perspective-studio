#!/usr/bin/env python3
"""Petit serveur local pour Perspective Studio : écoute uniquement sur 127.0.0.1, sert uniquement le dossier web/,
et demande au navigateur de ne rien garder en cache (pas besoin de recharger de force après une mise à jour).
Usage : python3 serve.py [port]   (arrêt : Ctrl+C)"""
import functools
import http.server
import pathlib
import sys

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8765
DOSSIER = pathlib.Path(__file__).resolve().parent / "web"


class SansCache(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()


if __name__ == "__main__":
    gestionnaire = functools.partial(SansCache, directory=str(DOSSIER))
    with http.server.ThreadingHTTPServer(("127.0.0.1", PORT), gestionnaire) as serveur:
        print(f"Perspective Studio : http://127.0.0.1:{PORT}/index.html  (Ctrl+C pour arrêter)")
        serveur.serve_forever()
