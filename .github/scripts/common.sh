BINDING_PATH=$(realpath $(dirname "$0"))/../..
RUBY_PATH=$BINDING_PATH/../ruby
RUBY_BUILD_PATH=$RUBY_PATH/build
RUBY_INSTALL_PATH=$RUBY_BUILD_PATH/install
RUSTUP_TOOLCHAIN=stable
# TODO: Switch to a fixed toolchain the next time we bump MSRV
#RUSTUP_TOOLCHAIN=`cat $BINDING_PATH/mmtk/rust-toolchain`
