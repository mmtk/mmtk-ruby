set -xe

source $(dirname "$0")/common.sh

export RUST_BACKTRACE=1

cd $RUBY_BUILD_PATH

echo "============ Bootstrape tests (btest) ($DEBUG_LEVEL) ============="
if test "$DEBUG_LEVEL" == "vanilla"; then
    make btest TESTOPTS="-v -j${CI_JOBS} ${YJIT_OPTS}"
else
    # Some test cases take too much time to run in debug mode, resulting in timeout. It is
    # because the default GC trigger (dynamic heap size) does not scale the heap size fast
    # enough to keep up with the allocation.  We simply run those tests with a fixed heap size
    # and exclude them from the rest of the btests.
    test_and_delete() {
        TEST_NAME=$1
        HEAP_SIZE=$2
        TEST_PATH=../bootstraptest/test_${TEST_NAME}.rb
        MMTK_GC_TRIGGER=FixedHeapSize:${HEAP_SIZE} make btest RUN_OPTS="--mmtk-plan=$CHOSEN_PLAN" TESTOPTS="-v $TEST_PATH"
        rm $TEST_PATH
    }

    test_and_delete eval 100m
    test_and_delete thread 400m

    # Run other btests in the regular way.
    make btest RUN_OPTS="--mmtk-plan=$CHOSEN_PLAN" TESTOPTS="-v -j${CI_JOBS} ${YJIT_OPTS}"
fi
