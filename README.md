# MMTk binding for Ruby

## Installation/build instructions

Changes to Ruby core language are avaliable [under my fork here](https://github.com/angussidney/ruby), on the branch `ruby_2_7`. This is required for compatiability with the mmtk-ruby binding.

Build MMTk, then copy `libmmtk_ruby.so` to `repos/ruby`.

```
# These flags are used to enable debugging symbols
CFLAGS="-O0 -ggdb3 -DUSE_THIRD_PARTY_HEAP" LD_LIBRARY_PATH=. ./configure prefix="/home/angusa/mmtk-ruby/repos/ruby/build"
make
make install
```

## TODO
 - Allocate all objects in the transient heap directly into MMTk, rather than letting it be managed by Ruby in a single heap allocated by MMTk
 - Add a runtime flag to enable MMTk/ruby, using environment variables. See #1
 - Rebase my changes onto the variable-sized objects heap (currently being developed by Shopify [here](https://github.com/Shopify/ruby/commits/mvh-pz-variable-width-allocation))
 - Use separate mutators for every thread for cache locality benefits (and correctness in case the GVL is ever removed)
 - Implement allocation fast paths