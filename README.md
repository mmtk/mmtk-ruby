# MMTk binding for Ruby

Build MMTk, then copy `libmmtk_ruby.so` to `repos/ruby`.

```
export CFLAGS="-DUSE_THIRD_PARTY_HEAP"
./configure --prefix=/home/angusa/mmtk-ruby/repos/ruby/build
make
make install
```