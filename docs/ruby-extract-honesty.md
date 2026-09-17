# Ruby extraction honesty (Tier 1)

FQN conventions:

- Nested constants: `Module::Class`
- Instance methods: `Module::Class#method`
- Class/singleton methods: `Module::Class.method`
- Constructors: `Module::Class.<init>` with `metadata.is_constructor: true`

Limits (static analysis only):

- No `$LOAD_PATH`, Bundler, or Zeitwerk resolution for `require`
- No Ruby method lookup, `super` target resolution, or refinements algebra
- Dynamic `send` / `method_missing` → `Calls` with `metadata.unresolved`
- `include` / `prepend` modeled as mixin `Extends`; `extend` as `Uses`
- Block/yield CFG uses nested sub-CFGs; yield edges are conservative
- Chef/Rails magic deferred to follow-up plugins

See also: [languages/ruby.md](languages/ruby.md) · [tier-1-language-support.md §8](tier-1-language-support.md#8-current-parity-snapshot-2026-07) · cold profile on `example/discourse` ([profile.md](internal/profile.md)).
