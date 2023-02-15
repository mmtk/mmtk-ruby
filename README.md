# MMTk binding for Ruby

This repository hosts the binding code for MMTk Ruby. 

In order for this binding to work, changes have been made to the Ruby core
language to support generic third party heaps. Eventually, the aim is to
upstream these changes. Until then, the modifications can be found [under our
fork here](https://github.com/mmtk/ruby), on the default branch named `mmtk`.

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
cargo build --release
popd
```

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

The `miniruby` executable should be able to execute simple Ruby programs.  You
can try the following commands:

```bash
# Run with vanilla Ruby GC
./miniruby -e 'puts "Hello world!"'

# Run with MMTk GC
./miniruby --mmtk -e 'puts "Hello world!"'

# You should see "MMTk" in the version string together with the current GC plan
./miniruby --version
./miniruby --mmtk --version
```

You can continue to build the full Ruby and install it with

```bash
make install -j
```

Then test it

```bash
./build/bin/ruby --mmtk --version
./build/bin/ruby --mmtk -e 'puts "Hello world!"'
```

### Debug build

**Building mmtk-ruby for debugging**

Remove the `--release` option to build `mmtk-ruby` for debug.  Note that the
Cargo build system is smart enough to let the result of debug build and release
build to coexist in the `target/debug` and `target/release` directories.

```bash
pushd mmtk-ruby/mmtk
cargo build
popd
```

**Building ruby for debugging**

Debugging can be enabled on Rust part (the `mmtk-ruby` repo) and the C part (the
`ruby` repo) independently.  The `ruby` repo determines whether to use the debug
or release version of `mmtk-ruby` using the `--with-mmtk-ruby-debug` flag of
`./configure`. By default, `./configure` searches for `libmmtk_ruby.so` in
`mmtk-ruby/mmtk/target/debug`, and the linker will subsequently link `miniruby`
and `ruby` to that `.so`. Add `--with-mmtk-ruby-debug` so it will search for
`libmmtk_ruby.so` in `mmtk-ruby/mmtk/target/debug`, instead.

Use the `--prefix` option to set the installation path to a local directory
instead of `/usr/local`.

Use the `--disable-install-doc` option to disable the generation of
documentations.  It can make the build process much faster.

Set the compiler option `-DRUBY_DEBUG=1` to enable most assertions in Ruby.

Set the compiler option `-DRUBY_DEVEL` and `-DUSE_RUBY_DEBUG_LOG=1` to enable
logging.

Add the `-g3 -O0` flag so that the debugger can see the values of most local
variables.  But if it is too slow, try `-O1` instead.

The following is an example of configuring for debugging.

```bash
./configure \
    --with-mmtk-ruby=../mmtk-ruby \
    --with-mmtk-ruby-debug \
    --prefix=$PWD/build \
    --disable-install-doc
    cppflags="-DRUBY_DEBUG=1 -DRUBY_DEVEL -DUSE_RUBY_DEBUG_LOG=1 -g3 -O0"
```

## Use Ruby with MMTk

### Selecting MMTk plans (GC algorithms)

Use the `--mmtk-plan` command line option to select the GC algorithm.  This
option implies `--mmtk`.  In MMTk, each "plan" corresponds to a GC algorithm.
Currently, supported plans include:

-   `NoGC`: Not doing GC at all.  When the heap is exhausted, it crashes.

-   `MarkSweep`: The classic mark-sweep algorithm.  Based on a free-list
    allocator, it never moves any object.

-   `Immix`: The [Immix algorithm][immix], a mark-region collector with
    opportunistic evacuation.  It moves objects from time to time to prevent
    the heap from being too fragmented.

[immix]: https://users.cecs.anu.edu.au/~steveb/pubs/papers/immix-pldi-2008.pdf

Example:

```bash
./miniruby --mmtk --mmtk-plan=Immix -e "puts 'Hello world!'"
```

### Adjusting heap size

By default, MMTk dynamically adjust the heap size between 1MiB and 80% of the
physical memory.  It is convenient for production settings. However, when doing
experiments, you may want to set the heap size to a fixed value so the GC
behaviour becomes more deterministic.

You can set the heap size using the `--mmtk-max-heap` command line option.

It accepts IEC suffixes "KiB", "MiB", "GiB" and "TiB".  Therefore, "16777216"
and "16MiB" are equivalent.

Example:

```bash
./miniruby --mmtk --mmtk-max-heap=512MiB -e "puts 'Hello world!'"
```

### Using the RUBYOPT environment variable

All of `--mmtk`, `--mmtk-plan` and `--mmtk-max-heap` options can be passed via
the `RUBYOPT` environment variable, too.

Example:

```bash
RUBYOPT='--mmtk-plan=Immix' ./miniruby --version
```

### MMTk-specific methods in the `GC::MMTk` module.

The `GC::MMTk` module contains methods specific to MMTk.

-   `GC::MMTk.plan_name`: Return the current MMTk plan.
-   `GC::MMTk.enabled?`: Return true if MMTk is enabled via the command line.
    Note that if the Ruby interpreter is not compiled with MMTk support
    (controlled by `./configure --with-mmtk-ruby`), the `GC::MMTk` module will
    not exist.  Use `defined? GC::MMTk` to check.
-   `GC::MMTk.harness_begin`: Call this before the interested part of a
    benchmark to start collecting statistic data.
-   `GC::MMTk.harness_end`: Call this before the interested part of a
    benchmark to stop collecting statistic data, and print the statistic data
    collected.

If you are running benchmarks, you should run the test case multiple times for
warming up, and measure the last iteration.  Call `harness_begin` and
`harness_end` before and after the last iteration.  The statistic data will be
printed to stderr.

## Test

### Bootstrap tests

When running `make btest`, use `RUN_OPTS` to pass additional parameters to the
`miniruby` program to enable MMTk.

```bash
make btest RUN_OPTS="--mmtk-plan=MarkSweep"
make btest RUN_OPTS="--mmtk-plan=Immix"
```

### All tests

<!-- FIXME: Check if anything below this still works. -->

To test Ruby, it is recommended that you add the `ADDITIONAL_EXCLUDES` option to exclude tests which make assumptions based on Ruby's current GC implementation, or are extremely memory intensive.

```
make test-all ADDITIONAL_EXCLUDES="--excludes-dir=./test/excludes/_third_party_heap"
```

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
