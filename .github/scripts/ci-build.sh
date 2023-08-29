set -xe

source $(dirname "$0")/common.sh

pushd $BINDING_PATH
    cd mmtk

    case $DEBUG_LEVEL in
        debug)
            cargo build
            ;;
        release)
            cargo build --release
            ;;
        *)
            echo "Unexpected debug level: $DEBUG_LEVEL"
            exit 1
            ;;
    esac
popd

NUM_OF_JOBS=2

pushd $RUBY_PATH

    ./autogen.sh

    mkdir -p $RUBY_BUILD_PATH
    cd $RUBY_BUILD_PATH

    case $DEBUG_LEVEL in
        debug)
            ../configure --with-mmtk-ruby=$BINDING_PATH --with-mmtk-ruby-debug --prefix=$RUBY_INSTALL_PATH --disable-install-doc cppflags='-g3 -O0 -DRUBY_DEBUG=1 -DRUBY_DEVEL -DUSE_RUBY_DEBUG_LOG=1'
            make miniruby -j $NUM_OF_JOBS
            ;;

        release)
            ../configure --with-mmtk-ruby=$BINDING_PATH --prefix=$RUBY_INSTALL_PATH --disable-install-doc cppflags='-g3'
            make install -j $NUM_OF_JOBS
            ;;

        *)
            echo "Unexpected debug level: $DEBUG_LEVEL"
            exit 1
            ;;
    esac
popd
