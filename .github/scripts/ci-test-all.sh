set -xe

source $(dirname "$0")/common.sh

cd $RUBY_BUILD_PATH

echo "============ Test all ($DEBUG_LEVEL) ============="
if test "$DEBUG_LEVEL" == "debug"; then
    echo "Skipping test-all for $DEBUG_LEVEL..."
else
    for test_case in $(cat $BINDING_PATH/ruby-test-cases.txt); do
        echo "-------[ Running $test_case ]-----[ DEBUG_LEVEL=$DEBUG_LEVEL ]-------"
        make test-all TESTS=$RUBY_PATH/test/ruby/$test_case RUN_OPTS="--mmtk-plan=Immix"
    done
fi

