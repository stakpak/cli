#!/bin/bash
set -e

# If the first argument is "docker", execute docker command
if [ "$1" = "docker" ]; then
    exec "$@"
else
    # Otherwise, execute stakpak command
    exec /usr/local/bin/stakpak "$@"
fi
