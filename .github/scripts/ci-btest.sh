set -xe

source $(dirname "$0")/common.sh

export RUST_BACKTRACE=1

cd $RUBY_BUILD_PATH

echo "============ Bootstrape tests (btest) ($DEBUG_LEVEL) ============="
if test "$DEBUG_LEVEL" == "vanilla"; then
    make btest TESTOPTS="-v -j${CI_JOBS}"
else
    make btest RUN_OPTS="--mmtk-plan=$CHOSEN_PLAN" TESTOPTS="-v -j${CI_JOBS}"
fi
