set -xe

source $(dirname "$0")/common.sh

export RUST_BACKTRACE=1

cd $RUBY_BUILD_PATH

echo "============ Test all ($DEBUG_LEVEL) ============="
case $DEBUG_LEVEL in
    debug)
        echo "Skipping test-all for $DEBUG_LEVEL..."
        ;;
    release)
        make test-all \
             RUN_OPTS="--mmtk-plan=$CHOSEN_PLAN ${YJIT_OPTS}" \
             TESTOPTS="-v --excludes-dir=../test/.excludes-mmtk -j${CI_JOBS}"
        ;;
    vanilla)
        # Temporarily disable test-all for the vanilla build.  Many TestGc test cases fail.
        # For example, heap_allocated_pages is increased after test_thrashing_for_young_objects.
        # But those failures only occur on GitHub CI.
        #make test-all TESTOPTS="-v -j${CI_JOBS}"
        ;;
    *)
        echo "Unexpected debug level: $DEBUG_LEVEL"
        exit 1
        ;;
esac

