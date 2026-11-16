---
name: web
description: Searches the web and reads its pages, with web_search(query) and web_fetch(url).
metadata:
  furb-word: web.py
  furb-interface: web.pyi
  furb-process: process.py
  furb-acts: fetch search
---
`web_search(query)` searches with the provider of the actor, and `web_fetch(url)` reads a page. Its contract is
`web.pyi`. Its process, `process.py`, reads each page, and answers each search of an actor whose provider has no
search of its own with a free search.
