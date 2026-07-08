#!/usr/bin/env bash
#
# Account transition-rules guard (flow boundary).
#
# Flow cores are the verified decision cores; all account mutation flows through
# them. For every account a flow core takes by `&mut`, it must carry a clause
#
#     #[cfg_attr(creusot, ensures((^arg).is_legal_transition_from(*arg)))]
#
# checked against the flow body and the contracts of the domain mutators it
# calls — so a state change that breaks the account's transition rules
# (immutable fields, monotonic counters, or legal status steps) fails to verify
# here (see each account's `is_legal_transition_from`). This guard fails the
# build for any flow `&mut` account argument missing its clause.
#
# Paragraph-based: a function's doc + attributes + signature form one blank-line
# block; for each `<arg>: &mut <Account>` in the block we require a matching
# `is_legal_transition_from(*<arg>)`.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
flows="$root/crates/ticketing/program/src/flows"

missing="$(
    for f in "$flows"/*.rs; do
        awk -v file="$f" '
            BEGIN { RS = ""; FS = "\n" }
            /fn / {
                name = ""
                for (i = 1; i <= NF; i++) if ($i ~ /[[:space:]]fn /) { name = $i; break }
                rest = $0
                while (match(rest, /[a-z_]+: &mut [A-Z][A-Za-z]*/)) {
                    arg = substr(rest, RSTART, RLENGTH)
                    sub(/:.*/, "", arg)
                    rest = substr(rest, RSTART + RLENGTH)
                    if ($0 !~ ("is_legal_transition_from\\(\\*" arg "\\)"))
                        print file ": " name "  [missing guard for `" arg "`]"
                }
            }
        ' "$f"
    done
)"

if [[ -n "$missing" ]]; then
    printf '%s\n' "$missing"
    {
        echo
        echo "error: the flow-core &mut account argument(s) above are missing their"
        echo "       is_legal_transition_from clause. Add, per &mut account arg:"
        echo "         #[cfg_attr(creusot, ensures((^arg).is_legal_transition_from(*arg)))]"
    } >&2
    exit 1
fi

echo "account transition-rules guard: every flow &mut account argument carries is_legal_transition_from"
