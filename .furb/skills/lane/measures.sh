#!/bin/sh
# The measures that steer the lane, from a tree and from GitHub, with no model. usage: measures.sh <tree> <base>
kit=$(cd "$(dirname "$0")" && pwd)
cd "$1" || exit 2
base=${2:-main}
git fetch -q origin main "$base" 2>/dev/null
from=$(git merge-base origin/main HEAD)
echo "head: $(git log --oneline -1 | cut -c1-90)"
echo "against main, all: $(git diff --shortstat "$from" HEAD)"
git diff --numstat "$from" HEAD | python3 -I "$kit/kinds.py"
echo "engine: $(.venv/bin/python script/tokens.py 2>&1 | grep engine | cut -c1-80)"
echo "held tests: $(grep -c . "$kit/ts-held.txt")"
pr=$(gh pr list --repo uael/furb --head "$base" --state open --json number --jq '.[0].number // empty' 2>/dev/null)
if [ -n "$pr" ]; then
  echo "checks of #$pr that are not green, by state: $(gh pr view "$pr" --repo uael/furb --json statusCheckRollup --jq '[.statusCheckRollup[]|{n:(.name//.context),c:(if (.conclusion//"")=="" then (.status//.state//"PENDING") else .conclusion end)}|select(.c!="SUCCESS" and .c!="SKIPPED")]|group_by(.c)|map(.[0].c+": "+(map(.n)|join(", ")))|join(" | ")')"
fi
echo "sentences of the contract of the kit: $(python3 -I -c 'import ast, sys; print(sum(len((ast.get_docstring(n) or "").splitlines()) for n in ast.parse(open(sys.argv[1]).read()).body if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef))))' "$kit/lane.pyi")"
