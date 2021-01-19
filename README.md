# MMTk binding for Ruby

## Installation/build instructions

Changes to Ruby core language are avaliable [under my fork here](https://github.com/angussidney/ruby), on the branch `ruby_2_7`. This is required for compatiability with the mmtk-ruby binding.

Build MMTk, then copy `libmmtk_ruby.so` to `repos/ruby`.

```bash
# Clone sources
git clone https://github.com/angussidney/mmtk-ruby.git
cd mmtk-ruby/repos
git clone https://github.com/angussidney/ruby.git
cd ../mmtk

# Build MMTk. Optionally edit Cargo.toml to use a local working copy
# of mmtk-core rather than a fresh cloned copy
export RUSTUP_TOOLCHAIN=nightly-2020-07-08
export DEBUG_LEVEL=fastdebug
cargo build --features nogc # Add --release to include optimisations
cp target/debug/libmmtk_ruby.so ../repos/ruby/

# Build Ruby with MMTk enabled
cd ../repos/ruby
export LD_LIBRARY_PATH=$PWD
# -O0/-ggdb3 flags are used for debugging
# Optionally add -DTHIRD_PARTY_HEAP_LIMIT=xxx to configure the heap size in bytes
CFLAGS="-O0 -ggdb3 -DUSE_THIRD_PARTY_HEAP -DUSE_TRANSIENT_HEAP=0" ./configure prefix="$PWD/build"
make install -j
echo "puts 'Hello, World'" > test.rb
./build/bin/ruby test.rb
```

## TODO
 - Add a runtime flag to enable MMTk/ruby, using environment variables. See #1
 - Rebase my changes onto the variable-sized objects heap (currently being developed by Shopify [here](https://github.com/Shopify/ruby/commits/mvh-pz-variable-width-allocation))
 - Use separate mutators for every thread for cache locality benefits (and correctness in case the GVL is ever removed)
 - Implement allocation fast paths
 - Fix tests