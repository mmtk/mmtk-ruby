set -xe

source $(dirname "$0")/common.sh

cd $RUBY_BUILD_PATH

echo "============ Test all ($DEBUG_LEVEL) ============="
case $DEBUG_LEVEL in
    debug)
        echo "Skipping test-all for $DEBUG_LEVEL..."
        ;;
    release)
        for test_case in $(cat $BINDING_PATH/ruby-test-cases.txt); do
            echo "-------[ Running $test_case ]-----[ DEBUG_LEVEL=$DEBUG_LEVEL ]-------"
            make test-all TESTS=$RUBY_PATH/test/ruby/$test_case RUN_OPTS="--mmtk-plan=Immix"
        done
        ;;
    *)
        echo "Unexpected debug level: $DEBUG_LEVEL"
        exit 1
        ;;
esac

