set -xe

source $(dirname "$0")/common.sh

export RUST_BACKTRACE=1

cd $RUBY_BUILD_PATH

echo "============ Bootstrape tests (btest) ($DEBUG_LEVEL) ============="
if test "$DEBUG_LEVEL" == "vanilla"; then
    make btest TESTOPTS="-v -j${CI_JOBS}"
else
    if test "$DEBUG_LEVEL" == "debug"; then
        # One test case in this file is taking too much time to run in debug mode, resulting in timeout.
        # It is because the default dynamic heap size does not work well with that pathological use case.
        # We simply run it with a fixed heap size and exclude it from the rest of the btests.
        TIMEOUT_TEST=../bootstraptest/test_eval.rb
        MMTK_GC_TRIGGER=FixedHeapSize:100m make btest RUN_OPTS="--mmtk-plan=$CHOSEN_PLAN" TESTOPTS="-v $TIMEOUT_TEST"
        rm $TIMEOUT_TEST
    fi
    make btest RUN_OPTS="--mmtk-plan=$CHOSEN_PLAN" TESTOPTS="-v -j${CI_JOBS}"
fi
