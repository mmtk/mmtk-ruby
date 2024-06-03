set -xe

. $(dirname "$0")/common.sh

export RUSTFLAGS="-D warnings"

pushd $BINDING_PATH/mmtk
cargo clippy
cargo clippy --release

cargo fmt -- --check
popd

find $BINDING_PATH \
    -name '*.rs' \
    -o -name '*.toml' \
    -o -name '*.md' \
    -o -name '*.sh' \
    -o -name '*.yml' | while read -r file; do
    $BINDING_PATH/.github/scripts/check-lineends.sh "$file"
done
