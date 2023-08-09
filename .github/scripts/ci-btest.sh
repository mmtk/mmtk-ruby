set -xe

source $(dirname "$0")/common.sh

cd $RUBY_PATH/build

echo "============ Bootstrape tests (btest) ($DEBUG_LEVEL) ============="
make btest RUN_OPTS=--mmtk-plan=Immix
