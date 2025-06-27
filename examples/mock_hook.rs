use ckb_mock_tx_types::{MockTransaction, Resource};
use ckb_script::types::DebugPrinter;
use ckb_script::types::VmId;
use ckb_types::packed::Byte32;
use ckb_vm::decoder::Decoder;
use ckb_vm::{Bytes, DefaultMachineRunner, Error as VmError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct HookCount {
    pub sum: u64,
}

impl<M> ckb_script::runner::Hook<M> for HookCount
where
    M: DefaultMachineRunner,
{
    fn init(_: &M) -> Self {
        HookCount::default()
    }

    fn init_by_exec(&mut self, _: &M) {}

    fn load_program(&mut self, _: &M, _: &Bytes, _: impl ExactSizeIterator<Item = Result<Bytes, VmError>>) {}

    fn step(&mut self, _: &mut M, _: &mut Decoder) -> Result<(), ckb_vm::Error> {
        self.sum += 1;
        Ok(())
    }
}

fn main() {
    let verifier_mock_tx: MockTransaction = {
        let buf = std::fs::read_to_string("res/spawn_cycle_mismatch_tx.json").unwrap();
        let repr_mock_tx: ckb_mock_tx_types::ReprMockTransaction = serde_json::from_str(&buf).unwrap();
        repr_mock_tx.into()
    };
    let tx = verifier_mock_tx.core_transaction();
    let dl = Resource::from_mock_tx(&verifier_mock_tx).unwrap();

    let config: ckb_script::runner::Config<
        Resource,
        DebugPrinter,
        ckb_script::runner::HookWraper<ckb_script::types::Machine, HookCount>,
    > = ckb_script::runner::Config {
        max_cycles: 100_000_000,
        syscall_generator: ckb_script::generate_ckb_syscalls,
        syscall_context: std::sync::Arc::new(|_: &Byte32, message: &str| {
            let message = message.trim_end_matches('\n');
            if message != "" {
                println!("{}", &format!("Script log: {}", message));
            }
        }),
        version: ckb_script::ScriptVersion::V2,
    };
    let runner = ckb_script::runner::Runner::new(tx, dl, config).unwrap();
    let mut scheduler =
        runner.get_scheduler_by_location("output".parse().unwrap(), 0, "type".parse().unwrap()).unwrap();

    let mut record = HashMap::<VmId, Arc<Mutex<HookCount>>>::new();
    while !scheduler.terminated() {
        scheduler.boot_root_vm_if_needed().unwrap();
        let currents = scheduler.iterate_prepare_machine().unwrap();
        let hook = record.entry(currents.0).or_insert(Default::default());
        currents.1.hook = hook.clone();
        scheduler.iterate().unwrap();
    }
    let expect: Vec<u64> = vec![
        3563545, 234481, 234481, 237034, 237109, 237109, 237109, 237109, 237109, 237109, 239724, 239724, 239724,
        239724, 239724, 239724, 239724,
    ];
    for i in 0..=16 {
        assert_eq!(record.get(&i).unwrap().lock().unwrap().sum, expect[i as usize]);
    }
}
