# MMTk binding for Ruby

This repository hosts the binding code for MMTk Ruby. 

In order for this binding to work, changes have been made to the Ruby core language to support generic third party heaps. Eventually, the aim is to upstream these changes. Until then, the modifications can be found [under my fork here](https://github.com/angussidney/ruby), on the branch `third-party-heap-2-7-2`. An overview of the changes can be viewed using [this diff](https://github.com/ruby/ruby/compare/ruby_2_7...angussidney:third-party-heap-2-7-2).

## Installation/build instructions

To build a copy of MMTk Ruby:

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
# Add --release to include optimisations. Highly recommended when
# not debugging (ields a huge performance increase)
cargo build --features nogc
cp target/debug/libmmtk_ruby.so ../repos/ruby/

# Build Ruby with MMTk enabled
cd ../repos/ruby
export LD_LIBRARY_PATH=$PWD
# -O0/-ggdb3 flags are used for debugging, remove for release
CFLAGS="-O0 -ggdb3 -DUSE_THIRD_PARTY_HEAP -DUSE_TRANSIENT_HEAP=0" ./configure prefix="$PWD/build"
# Note: this option is currently broken and allows unbounded heap sizes (bug in mmtk-core, #214)
export THIRD_PARTY_HEAP_LIMIT=1000000
make install -j
echo "puts 'Hello, World'" > test.rb
./build/bin/ruby test.rb
```

To test Ruby, it is recommended that you add the `ADDITIONAL_EXCLUDES` option to exclude tests which make assumptions based on Ruby's current GC implementation, or are extremely memory intensive.

```
make test-all ADDITIONAL_EXCLUDES="--excludes-dir=./test/excludes/_third_party_heap"
```

### Build with Vagrant

A Vagrant config for building on platforms other than Linux can be found here: https://github.com/chrisseaton/mmtk-ruby-macos

## Current status

Known working:
 - `./miniruby ./basictest/test.rb`
 - `make test`
 - Regular Ruby programs (note: this hasn't been tested on any extensive real-world programs, only <100 line dummy programs)
 - Basic Rails 5 app utilising a sqlite3 database:
    ```bash
    gem install rails -v 5.2.0
    rails new hello
    cd hello
    rails generate scaffold User name:string email:string
    rails db:migrate
    rails server

    # If you run into installation issues along the way, you may need to...
    gem install sqlite3
    gem install puma
    # ...and try again
    ```

Known issues:
 - `make test-all` fails. Many tests are GC implementation-dependent, so exclusion files have been created to ignore most of these. There are still >50 errors, but not all have been triaged or fixed yet.
 - GC implementation-specific modules (e.g. `ObjectSpace`, `GC`, `WeakRef`) and anything that relies on them (e.g. `Coverage`) are not supported. For now, there are no plans to implement these as many of the APIs are irrelevant (e.g. `GC.stat`); however some may be fixed in the future (e.g. `ObjectSpace.each_object`)
 - MJIT is not supported.

## TODO
 - Add a runtime flag to enable MMTk/ruby, using environment variables. See #1
 - Rebase my changes onto the variable-sized objects heap (currently being developed by Shopify [here](https://github.com/Shopify/ruby/commits/mvh-pz-variable-width-allocation))
 - Use separate mutators for every thread for cache locality benefits (and correctness in case the GVL is ever removed)
 - Implement allocation fast paths
 - Fix tests