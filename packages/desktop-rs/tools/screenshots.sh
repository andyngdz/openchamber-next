#!/usr/bin/env bash
# Renders every fixture state in a headless GPUI window and pairs each frame with its
# Electron reference, for review side by side.
#
#   tools/screenshots.sh            # every fixtures/states/*.json
#   tools/screenshots.sh empty.json # named states only
#
# Writes fixtures/screenshots/<state>.gpui.png, and where a reference exists,
# fixtures/screenshots/<state>.side-by-side.png with the React reference on the left.
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

states_dir=fixtures/states
reference_dir=fixtures/reference
out=fixtures/screenshots
mkdir -p "$out"

if ! command -v magick >/dev/null 2>&1; then
    printf 'screenshots: ImageMagick 7 (magick) is needed for the side-by-side images\n' >&2
    exit 1
fi

if (($# > 0)); then
    fixtures=()
    for name in "$@"; do
        fixtures+=("$states_dir/${name%.json}.json")
    done
else
    fixtures=("$states_dir"/*.json)
fi

failed=()
for fixture in "${fixtures[@]}"; do
    state=$(basename "$fixture" .json)
    rendered="$out/$state.gpui.png"
    printf 'rendering %s\n' "$state"
    if ! cargo run --quiet -p oc-app -- --state "$fixture" --screenshot "$rendered"; then
        failed+=("$state")
        continue
    fi

    reference="$reference_dir/$state.png"
    if [[ ! -f "$reference" ]]; then
        printf '  no React reference at %s; side-by-side skipped\n' "$reference"
        continue
    fi

    # The headless window is a test window: it renders at the test platform's scale, while
    # the reference is the 1440x900 content at 1x. Scale the frame down to the reference.
    size=$(magick identify -format '%wx%h' "$reference")
    scaled="$out/.$state.1x.png"
    magick "$rendered" -filter Lanczos -resize "$size!" -depth 8 "$scaled"
    magick montage \
        -background '#151313' -fill '#cdccc3' -font DejaVu-Sans -pointsize 22 \
        -label "$state · React reference" "$reference" \
        -label "$state · GPUI" "$scaled" \
        -tile 2x1 -geometry +0+0 -depth 8 "$out/$state.side-by-side.png"
    rm -f "$scaled"
    printf '  %s + %s\n' "$rendered" "$out/$state.side-by-side.png"
done

if ((${#failed[@]} > 0)); then
    printf 'screenshots: %d state(s) failed to render: %s\n' \
        "${#failed[@]}" "${failed[*]}" >&2
    exit 1
fi
