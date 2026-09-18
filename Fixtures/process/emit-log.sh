#!/bin/sh
set -eu

printf 'stdout-one\n'
sleep 0.03
printf 'stderr-one\n' >&2
sleep 0.03
printf '\342'
sleep 0.03
printf '\202\254 stdout-unicode\n'
sleep 0.03
printf '\346\274' >&2
sleep 0.03
printf '\242 stderr-unicode\n' >&2
