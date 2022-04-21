# MMTk binding for Ruby

This repository hosts the binding code for MMTk Ruby. 

In order for this binding to work, changes have been made to the Ruby core
language to support generic third party heaps. Eventually, the aim is to
upstream these changes. Until then, the modifications can be found [under our
fork here](https://github.com/mmtk/ruby), on the branch `third-party-heap`.

This repository is based on previous work of Angus Atkinson, and the original
repository can be found [here](https://github.com/angussidney/mmtk-ruby.git),
and the original Ruby fork can be found
[here](https://github.com/angussidney/ruby.git).

## Building/installation instructions

### Checkout repositories

You need to clone both the Ruby fork and the MMTk Ruby binding.  The location
does not matter.

```bash
git clone https://github.com/mmtk/ruby.git
git clone https://github.com/mmtk/mmtk-ruby.git
```

### Build the MMTk binding, first.

```bash
pushd mmtk-ruby/mmtk
cargo +nightly build --release
popd
```

*Note: mmtk-core uses the "stable" toolchain by default, but currently
mmtk-ruby depends on a feature only available in "nightly" Rust.*

This will give you a `libmmtk_ruby.so` in the `target/release` directory.

By default, `mmtk-ruby` uses the `mmtk` crate from the `master` branch of [its
official repository](https://github.com/mmtk/mmtk-core).  If you want to hack
the MMTk core itself, you can edit `mmtk-ruby/mmtk/Cargo.toml` to point to your
local repository.

### Then build our forked Ruby repository.

Configure.

```bash
cd ruby
./autogen.sh
./configure --with-mmtk-ruby=../mmtk-ruby --prefix=$PWD/build
```

Build a `miniruby` executable.  We need to set some environment variables
first.

```bash
export MMTK_PLAN=MarkSweep                  # Set the GC algorithm to MarkSweep
export THIRD_PARTY_HEAP_LIMIT=500000000     # 500M should be enough. If still out of memory, add more.

make miniruby -j
```

The `miniruby` executable should be able to execute simple Ruby programs.  You
can try the following command:

```bash
./miniruby -e 'puts "Hello world!"'
```

You can continue to build the full Ruby and install it with

```bash
make install -j
```

### Debug build

You can build with debug options enabled for easy debugging.

Remove the `--release` option to build `mmtk-ruby` for debug.  Note that the
Cargo build system is smart enough to let the result of debug build and release
build to coexist in the `target/debug` and `target/release` directories.

```bash
pushd mmtk-ruby/mmtk
cargo +nightly build
popd
```

By default, `./configure` searches for `libmmtk_ruby.so` in
`mmtk-ruby/mmtk/target/debug`, and the linker will subsequently link `miniruby`
and `ruby` to that `.so`. Add `--with-mmtk-ruby-debug` so it will search for
`libmmtk_ruby.so` in `mmtk-ruby/mmtk/target/debug`, instead.

Set the compiler option `-DRUBY_DEBUG=1` to enable most assertions in Ruby.

Add the `-O0` optimization flag so the debug can see the values of most local
variables.  But if it is too slow, try `-O1` instead.

The `--disable-install-doc` will disable the generation of documentations.  It
can make the build process much faster.

The following is an example of configuring for debugging.

```bash
./configure --with-mmtk-ruby=../mmtk-ruby --with-mmtk-ruby-debug cppflags='-DRUBY_DEBUG=1' optflags='-O0' --prefix=$PWD/build --disable-install-doc
```

Also note that you can use the release build of `mmtk-ruby` with a debug build
of `ruby` (with `optflags='-O0'` but without `--with-mmtk-ruby-debug`) and vice
versa (with `--with-mmtk-ruby-debug` and `optflags='-O1'` or higher).

## Test

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
 - `make test-rubyspec` is currently failing; need to find a way to exclude GC-specific specifications.
 - GC implementation-specific modules (e.g. `ObjectSpace`, `GC`, `WeakRef`) and anything that relies on them (e.g. `Coverage`) are not supported. For now, there are no plans to implement these as many of the APIs are irrelevant (e.g. `GC.stat`); however some may be fixed in the future (e.g. `ObjectSpace.each_object`)
 - MJIT is not supported.

## TODO
 - Add a runtime flag to enable MMTk/ruby, using environment variables. See #1
 - Rebase my changes onto the variable-sized objects heap (currently being developed by Shopify [here](https://github.com/Shopify/ruby/commits/mvh-pz-variable-width-allocation))
 - Use separate mutators for every thread for cache locality benefits (and correctness in case the GVL is ever removed)
 - Implement allocation fast paths
 - Fix tests


## Licensing

This work is dual-licensed under the MIT and Apache licenses, to be compatible with the MMTk-Core project. See the license notices in the root of this repository for further details.
