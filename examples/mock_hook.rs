use ckb_mock_tx_types::{MockTransaction, Resource};
use ckb_script::ROOT_VM_ID;
use ckb_script::types::{DebugPrinter, VmId};
use ckb_types::packed::Byte32;
use ckb_vm::decoder::Decoder;
use ckb_vm::{Bytes, DefaultMachineRunner, Error as VmError};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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

    fn meld(&mut self, _: &mut Self) {}

    fn load_program(&mut self, _: &Bytes, _: impl ExactSizeIterator<Item = Result<Bytes, VmError>>) {}

    fn step(&mut self, _: &mut Decoder, _: &mut M) -> Result<(), ckb_vm::Error> {
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

    let mut records = HashMap::<VmId, Rc<RefCell<HookCount>>>::new();
    while !scheduler.terminated() {
        if scheduler.consumed_cycles() != 0 {
            let (id, vm) = scheduler.iterate_prepare_machine().unwrap();
            if let Some(data) = records.get(&id) {
                vm.hook = data.clone();
            }
        }
        let result = scheduler.iterate().unwrap();
        if result.executed_vm == ROOT_VM_ID {
            let hook = scheduler.peek(&result.executed_vm, |m| Ok(m.hook.clone()), |&_, &_| unreachable!()).unwrap();
            records.insert(result.executed_vm, hook);
        }
        if result.executed_vm != ROOT_VM_ID && scheduler.state(&result.executed_vm).is_some() {
            let hook = scheduler.peek(&result.executed_vm, |m| Ok(m.hook.clone()), |&_, &_| unreachable!()).unwrap();
            records.insert(result.executed_vm, hook);
        }
    }
    for k in 0..=16 {
        println!("{:?} {:?}", k, records.get(&k).unwrap().borrow().sum);
    }
}
