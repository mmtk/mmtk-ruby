# Ruby binding-specific timeline visualizer extensions

Scripts in this directory extends the eBPF timeline visualizer tools in the
[mmtk-core](https://github.com/mmtk/mmtk-core/) repository to add more information about work
packets defined in the mmtk-ruby binding.

Read `mmtk-core/tools/tracing/timeline/README.md` for the basic usage of the tools, and read
`mmtk-core/tools/tracing/timeline/EXTENSION.md` for details about extensions.

## Examples:

To capture a trace with Ruby-specific information:

```
/path/to/mmtk-core/tools/tracing/timeline/capture.py \
    -x /path/to/mmtk-ruby/tools/tracing/timeline/capture_ruby.bt \
    -m /path/to/mmtk-ruby/mmtk/target/release/libmmtk_ruby.so \
    > my-execution.log
```

To convert the log into the JSON format, with Ruby-specific information added to the timeline
blocks:

```
/path/to/mmtk-core/tools/tracing/timeline/visualize.py \
    -x /path/to/mmtk-ruby/tools/tracing/timeline/visualize_ruby.bt \
    my-execution.log
```

It will generate `my-execution.log.json.gz` which can be loaded into [Perfetto UI].

[Perfetto UI]: https://www.ui.perfetto.dev/
