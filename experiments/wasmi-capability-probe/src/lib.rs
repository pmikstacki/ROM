//! Standalone engine fixture. No ROM dependency or maintained integration.
#[cfg(test)]
mod tests {
    use wasmi::{Config, Engine, Instance, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};
    const PAGE: usize = 65_536;
    fn engine() -> Engine {
        let mut config = Config::default();
        config.consume_fuel(true);
        Engine::new(&config)
    }
    fn store(engine: &Engine) -> Store<StoreLimits> {
        let limits = StoreLimitsBuilder::new()
            .memory_size(PAGE)
            .memories(1)
            .instances(1)
            .tables(1)
            .table_elements(16)
            .build();
        let mut store = Store::new(engine, limits);
        store.limiter(|limits| limits);
        store.set_fuel(10_000).unwrap();
        store
    }
    fn instantiate(wat: &str) -> (Store<StoreLimits>, Instance) {
        let engine = engine();
        let module = Module::new(&engine, wat).unwrap();
        let mut store = store(&engine);
        let instance = Linker::new(&engine)
            .instantiate_and_start(&mut store, &module)
            .unwrap();
        (store, instance)
    }
    #[test]
    fn infinite_guest_loop_exhausts_fuel() {
        let (mut store, instance) = instantiate("(module (func (export \"run\") (loop br 0)))");
        let error = instance
            .get_typed_func::<(), ()>(&store, "run")
            .unwrap()
            .call(&mut store, ())
            .unwrap_err();
        assert!(format!("{error:?}").contains("OutOfFuel"), "{error:?}");
    }
    #[test]
    fn unavailable_host_import_prevents_instantiation() {
        let engine = engine();
        let module = Module::new(
            &engine,
            "(module (import \"host\" \"database-write\" (func)))",
        )
        .unwrap();
        let error = Linker::new(&engine)
            .instantiate_and_start(&mut store(&engine), &module)
            .unwrap_err();
        assert!(format!("{error:?}").contains("database-write"), "{error:?}");
    }
    #[test]
    fn guest_start_function_is_also_fuel_bounded() {
        let engine = engine();
        let module =
            Module::new(&engine, "(module (func $start (loop br 0)) (start $start))").unwrap();
        let error = Linker::new(&engine)
            .instantiate_and_start(&mut store(&engine), &module)
            .unwrap_err();
        assert!(format!("{error:?}").contains("OutOfFuel"), "{error:?}");
    }
    #[test]
    fn native_import_work_is_not_metered_as_guest_instructions() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        let engine = engine();
        let module = Module::new(
            &engine,
            "(module (import \"host\" \"work\" (func $work)) (func (export \"run\") call $work))",
        )
        .unwrap();
        let work = Arc::new(AtomicUsize::new(0));
        let called = work.clone();
        let mut linker = Linker::new(&engine);
        linker
            .func_wrap("host", "work", move || {
                for _ in 0..100_000 {
                    called.fetch_add(1, Ordering::Relaxed);
                }
            })
            .unwrap();
        let mut store = store(&engine);
        let instance = linker.instantiate_and_start(&mut store, &module).unwrap();
        instance
            .get_typed_func::<(), ()>(&store, "run")
            .unwrap()
            .call(&mut store, ())
            .unwrap();
        assert_eq!(work.load(Ordering::Relaxed), 100_000);
        assert!(store.get_fuel().unwrap() > 0);
    }
    #[test]
    fn memory_growth_limit_is_enforced_before_growth() {
        let (mut store, instance) = instantiate(
            "(module (memory (export \"memory\") 1 10) (func (export \"grow\") (result i32) i32.const 1 memory.grow))",
        );
        let grow = instance.get_typed_func::<(), i32>(&store, "grow").unwrap();
        assert_eq!(grow.call(&mut store, ()).unwrap(), -1);
        assert_eq!(
            instance
                .get_memory(&store, "memory")
                .unwrap()
                .data(&store)
                .len(),
            PAGE
        );
    }
    #[derive(Debug, serde::Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Proposal {
        quantity: u64,
    }
    #[derive(Debug, PartialEq)]
    enum Rejected {
        TooLarge,
        InvalidRange,
        InvalidOutput,
        Trap,
    }
    fn proposal(
        store: &mut Store<StoreLimits>,
        instance: &Instance,
        cap: usize,
    ) -> Result<Proposal, Rejected> {
        let ptr = instance
            .get_typed_func::<(), i32>(&*store, "ptr")
            .unwrap()
            .call(&mut *store, ())
            .map_err(|_| Rejected::Trap)? as u32 as usize;
        let len = instance
            .get_typed_func::<(), i32>(&*store, "len")
            .unwrap()
            .call(&mut *store, ())
            .map_err(|_| Rejected::Trap)? as u32 as usize;
        // Admission precedes allocation and guest-memory slicing.
        if len > cap {
            return Err(Rejected::TooLarge);
        }
        let end = ptr.checked_add(len).ok_or(Rejected::InvalidRange)?;
        let memory = instance.get_memory(&*store, "memory").unwrap();
        let bytes = memory
            .data(&*store)
            .get(ptr..end)
            .ok_or(Rejected::InvalidRange)?;
        let decoded: Proposal =
            serde_json::from_slice(bytes).map_err(|_| Rejected::InvalidOutput)?;
        if decoded.quantity > 100 {
            return Err(Rejected::InvalidOutput);
        }
        Ok(decoded)
    }
    fn output_module(bytes: &[u8], ptr: u32, len: u32) -> String {
        let escaped = bytes
            .iter()
            .map(|b| format!("\\{b:02x}"))
            .collect::<String>();
        format!(
            "(module (memory (export \"memory\") 1) (data (i32.const 0) \"{escaped}\") (func (export \"ptr\") (result i32) i32.const {ptr}) (func (export \"len\") (result i32) i32.const {len}))"
        )
    }
    #[test]
    fn bounded_proposal_is_validated_without_any_mutation_capability() {
        let bytes = br#"{"quantity":7}"#;
        let (mut store, instance) = instantiate(&output_module(bytes, 0, bytes.len() as u32));
        assert_eq!(
            proposal(&mut store, &instance, 64),
            Ok(Proposal { quantity: 7 })
        );
    }
    #[test]
    fn oversized_output_is_rejected_before_copying() {
        let (mut store, instance) = instantiate(&output_module(b"", 0, u32::MAX));
        assert_eq!(proposal(&mut store, &instance, 64), Err(Rejected::TooLarge));
    }
    #[test]
    fn out_of_range_output_is_rejected() {
        let (mut store, instance) = instantiate(&output_module(b"", u32::MAX, 8));
        assert_eq!(
            proposal(&mut store, &instance, 64),
            Err(Rejected::InvalidRange)
        );
    }
    #[test]
    fn invalid_utf8_duplicate_unknown_or_semantically_invalid_proposals_fail() {
        for bytes in [
            &b"\xff"[..],
            &br#"{"quantity":7,"quantity":8}"#[..],
            &br#"{"quantity":7,"secret":"leak"}"#[..],
            &br#"{"quantity":101}"#[..],
            &br#"{"quantity":null}"#[..],
        ] {
            let (mut store, instance) = instantiate(&output_module(bytes, 0, bytes.len() as u32));
            assert_eq!(
                proposal(&mut store, &instance, 64),
                Err(Rejected::InvalidOutput)
            );
        }
    }

    // Opt-in micro-observation, not a production workload or comparative benchmark.
    #[test]
    #[ignore = "release-only timing observation"]
    fn repeated_release_cost_observation() {
        use std::{hint::black_box, time::Instant};
        let bytes = br#"{"quantity":7}"#;
        let wat = output_module(bytes, 0, bytes.len() as u32);
        let engine = engine();
        let module = Module::new(&engine, &wat).unwrap();
        let linker = Linker::new(&engine);
        let (mut shared, instance) = instantiate(&wat);
        let ptr = instance.get_typed_func::<(), i32>(&shared, "ptr").unwrap();
        let len = instance.get_typed_func::<(), i32>(&shared, "len").unwrap();
        // Force Wasmi's lazy function translation before warm measurements.
        assert_eq!(proposal(&mut shared, &instance, 64).unwrap().quantity, 7);
        println!("sample,operation,iterations,total_ns,ns_per_iteration");
        for sample in 0..5 {
            let start = Instant::now();
            for _ in 0..200 {
                black_box(Module::new(&engine, black_box(&wat)).unwrap());
            }
            let elapsed = start.elapsed().as_nanos();
            println!(
                "{sample},wat_parse_validate_module,200,{elapsed},{}",
                elapsed / 200
            );

            let start = Instant::now();
            for _ in 0..200 {
                let mut fresh = store(&engine);
                black_box(linker.instantiate_and_start(&mut fresh, &module).unwrap());
            }
            let elapsed = start.elapsed().as_nanos();
            println!(
                "{sample},fresh_store_instantiate_cached_module,200,{elapsed},{}",
                elapsed / 200
            );

            shared.set_fuel(1_000_000).unwrap();
            let start = Instant::now();
            for _ in 0..2_000 {
                black_box(proposal(&mut shared, &instance, 64).unwrap());
            }
            let elapsed = start.elapsed().as_nanos();
            println!(
                "{sample},warm_exports_lookup_calls_validate,2000,{elapsed},{}",
                elapsed / 2_000
            );

            shared.set_fuel(1_000_000).unwrap();
            let start = Instant::now();
            for _ in 0..2_000 {
                black_box(ptr.call(&mut shared, ()).unwrap());
                black_box(len.call(&mut shared, ()).unwrap());
            }
            let elapsed = start.elapsed().as_nanos();
            println!(
                "{sample},warm_cached_typed_export_calls_only,2000,{elapsed},{}",
                elapsed / 2_000
            );

            let start = Instant::now();
            for _ in 0..2_000 {
                assert!(bytes.len() <= 64);
                let decoded: Proposal = serde_json::from_slice(black_box(bytes)).unwrap();
                assert!(decoded.quantity <= 100);
                black_box(decoded);
            }
            let elapsed = start.elapsed().as_nanos();
            println!(
                "{sample},host_json_schema_semantics_only,2000,{elapsed},{}",
                elapsed / 2_000
            );
        }
    }
}
