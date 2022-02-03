use deno_core::Extension;

use deno_bench_util::bench_or_profile;
use deno_bench_util::bencher::{benchmark_group, Bencher};
use deno_bench_util::{bench_js_async, bench_js_sync};
use deno_web::BlobStore;

struct Permissions;

impl deno_timers::TimersPermission for Permissions {
  fn allow_hrtime(&mut self) -> bool {
    true
  }
  fn check_unstable(
    &self,
    _state: &deno_core::OpState,
    _api_name: &'static str,
  ) {
  }
}

fn setup() -> Vec<Extension> {
  vec![
    deno_webidl::init(),
    deno_url::init(),
    deno_web::init(BlobStore::default(), None),
    deno_timers::init::<Permissions>(),
    Extension::builder()
    .js(vec![
      ("setup",
        Box::new(|| Ok(r#"
        const { opNow, setTimeout, handleTimerMacrotask } = globalThis.__bootstrap.timers;
        Deno.core.setMacrotaskCallback(handleTimerMacrotask);
        "#.to_owned())),
      ),
    ])
    .state(|state| {
      state.put(Permissions{});
      Ok(())
    })
    .build()
  ]
}

fn bench_op_now(b: &mut Bencher) {
  bench_js_sync(b, r#"opNow();"#, setup);
}

fn bench_set_timeout(b: &mut Bencher) {
  bench_js_async(b, r#"setTimeout(() => {}, 0);"#, setup);
}

benchmark_group!(benches, bench_op_now, bench_set_timeout,);
bench_or_profile!(benches);
