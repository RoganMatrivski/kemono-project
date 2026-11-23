#!/bin/sh
# Validates schema/*.yaml as OpenAPI. Uses a throwaway venv; needs python3 (e.g. via mise).
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
venv="${TMPDIR:-/tmp}/kemono-schema-venv"
if [ ! -x "$venv/bin/python" ]; then
  python3 -m venv "$venv"
  "$venv/bin/pip" install -q pyyaml openapi-spec-validator
fi
for f in "$root"/schema/*.yaml; do
  "$venv/bin/python" -I -c '
import sys, yaml
from openapi_spec_validator import validate
d = yaml.safe_load(open(sys.argv[1])); validate(d)
print("ok:", sys.argv[1], len(d["paths"]), "paths")' "$f"
done
