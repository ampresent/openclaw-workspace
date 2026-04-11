#!/bin/bash
# Example: Setting up Docker with nix-evo API

AGENT="http://127.0.0.1:7890"

CONFIG='
virtualisation.docker = {
  enable = true;
  autoPrune.enable = true;
};
users.users.admin.extraGroups = [ "docker" ];
'

echo "=== Validate Docker config ==="
curl -s -X POST "$AGENT/api/config/validate" \
  -H "Content-Type: application/json" \
  -d "{\"config\": \"$CONFIG\"}" | jq '.summary'
