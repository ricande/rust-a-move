#!/bin/sh
# Updates a game made from the RustyHusky template to a newer template:
#   ./scripts/update-framework.sh /path/to/RustyHusky
# The path is a checkout of the template at the version you want. The
# framework's files are copied over; the game's own files are kept. In
# src/main.rs and src/config.rs, which mix both, the template's version is
# used with the game's own lines put back (its `mod` lines and `ActiveGame`;
# its name, id, environment prefix and video limit). Then the checks run.
# Review `git diff` afterwards and note the update in CHANGELOG.md.
set -eu
cd "$(dirname "$0")/.."
template=${1:?usage: $0 /path/to/RustyHusky}
[ -f "$template/src/app.rs" ] && [ -f "$template/scripts/new-game.sh" ] \
    || { echo "$template is not a RustyHusky checkout" >&2; exit 1; }
[ -z "$(git status --porcelain)" ] \
    || { echo "commit or stash your changes first" >&2; exit 1; }

# The framework. A new framework file must be added here.
framework_src="app audio capture game i18n pacing screen screens script settings ui video view"
for f in $framework_src; do
    cp "$template/src/$f.rs" "src/$f.rs"
done
for f in scripts/check-vendored-miniquad.sh scripts/update-framework.sh \
    scripts/new-game.sh scripts/ui_tour.txt scripts/window_check.txt \
    docs/ARCHITECTURE.md docs/NEW_GAME.md docs/UPSTREAM_FIXES.md AGENTS.md \
    .github/workflows/ci.yml LICENSE; do
    mkdir -p "$(dirname "$f")"
    cp "$template/$f" "$f"
done
rm -rf vendor LICENSES
cp -r "$template/vendor" "$template/LICENSES" .

# src/main.rs: the template's, with the game's modules and ActiveGame.
mods=$(grep -E '^mod [a-z_0-9]+;$' src/main.rs | while read -r line; do
    name=${line#mod }; name=${name%;}
    case " $framework_src config example " in *" $name "*) ;; *) echo "$line" ;; esac
done)
active=$(grep -E '^type ActiveGame = ' src/main.rs)
awk -v mods="$mods" -v active="$active" '
    /^mod example;$/ { if (mods != "") print mods; else print; next }
    /^type ActiveGame = / { print active; next }
    { print }' "$template/src/main.rs" > src/main.rs.new
mv src/main.rs.new src/main.rs

# src/config.rs: the template's, with the game's values.
for name in APP_NAME APP_ID ENV_PREFIX MAX_VIDEO_SECONDS; do
    line=$(grep -E "^pub const $name: " src/config.rs)
    printf '%s\n' "$line" > "src/.$name"
done
cp "$template/src/config.rs" src/config.rs
for name in APP_NAME APP_ID ENV_PREFIX MAX_VIDEO_SECONDS; do
    awk -v name="$name" -v file="src/.$name" '
        BEGIN { getline repl < file }
        $0 ~ "^pub const " name ": " { print repl; next }
        { print }' src/config.rs > src/config.rs.new
    mv src/config.rs.new src/config.rs
    rm "src/.$name"
done

cargo fmt
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
echo
echo "Updated. Review 'git diff', then compare these with the template by hand:"
for f in Cargo.toml THIRD_PARTY_NOTICES.md; do
    cmp -s "$template/$f" "$f" || echo "  $f"
done
echo "and add the update to CHANGELOG.md."
