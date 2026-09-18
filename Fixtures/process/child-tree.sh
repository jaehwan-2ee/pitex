#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: child-tree.sh TEMP_DIRECTORY" >&2
    exit 64
fi

record_directory=$1
if [ ! -d "$record_directory" ]; then
    echo "record directory does not exist" >&2
    exit 65
fi

printf '%s\n' "$$" > "$record_directory/parent.pid"
(
    printf '%s\n' "$$" > "$record_directory/child.pid"
    (
        printf '%s\n' "$$" > "$record_directory/grandchild.pid"
        exec sleep 300
    ) &
    grandchild=$!
    printf '%s\n' "$grandchild" > "$record_directory/grandchild.pid"
    wait "$grandchild"
) &
child=$!
printf '%s\n' "$child" > "$record_directory/child.pid"
wait "$child"
